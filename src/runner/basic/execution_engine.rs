//! Execution orchestration and feature insertion logic.

use std::{
    ops::ControlFlow,
    panic,
    thread,
    time::Instant,
};

use futures::{
    Stream, StreamExt as _,
    channel::{mpsc, oneshot},
    future,
    pin_mut,
    stream,
};

#[cfg(feature = "tracing")]
use crate::tracing::{Collector as TracingCollector, SpanCloseWaiter};
use crate::{
    Event, World,
    event::{self, HookType},
    feature::Ext as _,
    future::{FutureExt as _, select_with_biased_first},
    parser, step,
};

use super::{
    cli_and_types::{Cli, RetryOptionsFn, ScenarioType},
    executor::Executor,
    scenario_storage::{Features, FinishedRulesAndFeatures},
    supporting_structures::ScenarioId,
};

/// Stores [`Feature`]s for later use by [`execute()`].
///
/// [`Feature`]: gherkin::Feature
pub(super) async fn insert_features<W, S, F>(
    into: Features,
    features_stream: S,
    which_scenario: F,
    retries: RetryOptionsFn,
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
    cli: Cli,
    fail_fast: bool,
) where
    S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
{
    let mut features = 0;
    let mut rules = 0;
    let mut scenarios = 0;
    let mut steps = 0;
    let mut parser_errors = 0;

    pin_mut!(features_stream);
    while let Some(feat) = features_stream.next().await {
        match feat {
            Ok(f) => {
                features += 1;
                rules += f.rules.len();
                scenarios += f.count_scenarios();
                steps += f.count_steps();

                into.insert(f, &which_scenario, &retries, &cli).await;
            }
            Err(e) => {
                parser_errors += 1;

                // If the receiver end is dropped, then no one listens for the
                // events, so we can just stop from here.
                if sender.unbounded_send(Err(e)).is_err() || fail_fast {
                    break;
                }
            }
        }
    }

    drop(sender.unbounded_send(Ok(Event::new(
        event::Cucumber::ParsingFinished {
            features,
            rules,
            scenarios,
            steps,
            parser_errors,
        },
    ))));

    into.finish();
}

/// Retrieves [`Feature`]s and executes them.
///
/// # Events
///
/// - [`Scenario`] events are emitted by [`Executor`].
/// - If [`Scenario`] was first or last for particular [`Rule`] or [`Feature`],
///   emits starting or finishing events for them.
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
#[cfg_attr(
    feature = "tracing",
    expect(clippy::too_many_arguments, reason = "needs refactoring")
)]
pub(super) async fn execute<W, Before, After>(
    features: Features,
    max_concurrent_scenarios: Option<usize>,
    collection: step::Collection<W>,
    event_sender: mpsc::UnboundedSender<
        parser::Result<Event<event::Cucumber<W>>>,
    >,
    before_hook: Option<Before>,
    after_hook: Option<After>,
    fail_fast: bool,
    #[cfg(feature = "tracing")] mut logs_collector: Option<TracingCollector>,
    #[cfg(feature = "observability")] observers: std::sync::Arc<std::sync::Mutex<crate::observer::ObserverRegistry<W>>>,
) where
    W: World,
    Before: 'static
        + for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> futures::future::LocalBoxFuture<'a, ()>,
    After: 'static
        + for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> futures::future::LocalBoxFuture<'a, ()>,
{
    // Those panic hook shenanigans are done to avoid console messages like
    // "thread 'main' panicked at ..."
    //
    // 1. We obtain the current panic hook and replace it with an empty one.
    // 2. We run tests, which can panic. In that case we pass all panic info
    //    down the line to the Writer, which will print it at a right time.
    // 3. We restore original panic hook, because suppressing all panics doesn't
    //    sound like a very good idea.
    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let (finished_sender, finished_receiver) = mpsc::unbounded();
    let mut storage = FinishedRulesAndFeatures::new(finished_receiver);
    let executor = Executor::new(
        collection,
        before_hook,
        after_hook,
        event_sender,
        finished_sender,
        features.clone(),
        #[cfg(feature = "observability")]
        observers,
    );

    executor.send_event(event::Cucumber::Started);

    #[cfg(feature = "tracing")]
    let waiter = logs_collector
        .as_ref()
        .map(TracingCollector::scenario_span_event_waiter);

    let mut started_scenarios = ControlFlow::Continue(max_concurrent_scenarios);
    let mut run_scenarios = stream::FuturesUnordered::new();
    loop {
        let (runnable, sleep) = features
            .get(started_scenarios.continue_value().unwrap_or(Some(0)))
            .await;
        if run_scenarios.is_empty() && runnable.is_empty() {
            if features.is_finished(started_scenarios.is_break()).await {
                break;
            }

            // To avoid busy-polling of `Features::get()`, in case there are no
            // scenarios that are running or scheduled for execution, we spawn a
            // thread, that sleeps for minimal deadline of all retried
            // scenarios.
            // TODO: Replace `thread::spawn` with async runtime agnostic sleep,
            //       once it's available.
            if let Some(dur) = sleep {
                let (sender, receiver) = oneshot::channel();
                drop(thread::spawn(move || {
                    thread::sleep(dur);
                    sender.send(())
                }));
                _ = receiver.await.ok();
            }

            continue;
        }

        let started = storage.start_scenarios(&runnable);
        executor.send_all_events(started);

        {
            #[cfg(feature = "tracing")]
            let forward_logs = {
                if let Some(coll) = logs_collector.as_mut() {
                    coll.start_scenarios(&runnable);
                }
                async {
                    #[expect( // intentional
                        clippy::infinite_loop,
                        reason = "cannot annotate `async` block with `-> !`"
                    )]
                    loop {
                        while let Some(logs) = logs_collector
                            .as_mut()
                            .and_then(TracingCollector::emitted_logs)
                        {
                            executor.send_all_events(logs);
                        }
                        future::ready(()).then_yield().await;
                    }
                }
            };
            #[cfg(feature = "tracing")]
            pin_mut!(forward_logs);
            #[cfg(not(feature = "tracing"))]
            let forward_logs = future::pending();

            if let ControlFlow::Continue(Some(sc)) = &mut started_scenarios {
                *sc -= runnable.len();
            }

            for (id, f, r, s, ty, retry_options) in runnable {
                run_scenarios.push(
                    executor
                        .run_scenario(
                            id,
                            f,
                            r,
                            s,
                            ty,
                            retry_options,
                            #[cfg(feature = "tracing")]
                            waiter.as_ref(),
                        )
                        .then_yield(),
                );
            }

            let (finished_scenario, _) =
                select_with_biased_first(forward_logs, run_scenarios.next())
                    .await
                    .factor_first();
            if finished_scenario.is_some() {
                if let ControlFlow::Continue(Some(sc)) = &mut started_scenarios
                {
                    *sc += 1;
                }
            }
        }

        while let Ok(Some((id, feat, rule, scenario_failed, retried))) =
            storage.finished_receiver.try_next()
        {
            if let Some(rule) = rule {
                if let Some(f) =
                    storage.rule_scenario_finished(feat.clone(), rule, retried)
                {
                    executor.send_event(f);
                }
            }
            if let Some(f) = storage.feature_scenario_finished(feat, retried) {
                executor.send_event(f);
            }
            #[cfg(feature = "tracing")]
            {
                if let Some(coll) = logs_collector.as_mut() {
                    coll.finish_scenario(id);
                }
            }
            #[cfg(not(feature = "tracing"))]
            let _: ScenarioId = id;

            if fail_fast && scenario_failed && !retried {
                started_scenarios = ControlFlow::Break(());
            }
        }
    }

    // This is done in case of `fail_fast: true`, when not all `Scenario`s might
    // be executed.
    executor.send_all_events(storage.finish_all_rules_and_features());

    executor.send_event(event::Cucumber::Finished);

    panic::set_hook(hook);
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use std::sync::{Arc, Mutex};
    use crate::test_utils::common::TestWorld;
    use crate::runner::basic::RetryOptions;

    // Using common TestWorld from test_utils

    #[tokio::test]
    async fn test_insert_features_empty_stream() {
        let features = Features::default();
        let (sender, mut receiver) = mpsc::unbounded();
        let cli = Cli::default();
        
        let which_scenario = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario| {
            ScenarioType::Concurrent
        };
        
        let retry_fn = Arc::new(|_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario, _: &Cli| -> Option<RetryOptions> { None });
        
        insert_features(
            features.clone(),
            stream::empty(),
            which_scenario,
            retry_fn,
            sender,
            cli,
            false,
        ).await;
        
        // Should receive ParsingFinished event
        let event: event::Event<event::Cucumber<TestWorld>> = receiver.next().await.unwrap().unwrap();
        match event.value {
            event::Cucumber::ParsingFinished { features, rules, scenarios, steps, parser_errors } => {
                assert_eq!(features, 0);
                assert_eq!(rules, 0);
                assert_eq!(scenarios, 0);
                assert_eq!(steps, 0);
                assert_eq!(parser_errors, 0);
            }
            _ => panic!("Expected ParsingFinished event"),
        }
        
        // No more events
        assert!(receiver.next().await.is_none());
    }

    #[tokio::test]
    async fn test_insert_features_with_error() {
        let features = Features::default();
        let (sender, mut receiver) = mpsc::unbounded();
        let cli = Cli::default();
        
        let which_scenario = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario| {
            ScenarioType::Concurrent
        };
        
        let retry_fn = Arc::new(|_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario, _: &Cli| -> Option<RetryOptions> { None });
        
        let error_stream = stream::once(async { 
            Err(parser::Error::Parsing(std::sync::Arc::new(gherkin::ParseFileError::Reading {
                path: std::path::PathBuf::from("test.feature"),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "Test file not found"),
            }))) 
        });
        
        insert_features(
            features.clone(),
            error_stream,
            which_scenario,
            retry_fn,
            sender,
            cli,
            false,
        ).await;
        
        // Should receive error first
        let error_event = receiver.next().await.unwrap();
        assert!(error_event.is_err());
        
        // Then ParsingFinished event
        let event: Event<event::Cucumber<TestWorld>> = receiver.next().await.unwrap().unwrap();
        match event.value {
            event::Cucumber::ParsingFinished { parser_errors, .. } => {
                assert_eq!(parser_errors, 1);
            }
            _ => panic!("Expected ParsingFinished event"),
        }
    }

    #[tokio::test]
    async fn test_execute_with_empty_features() {
        let features = Features::default();
        features.finish(); // Mark as finished
        
        let (sender, mut receiver) = mpsc::unbounded();
        let collection = step::Collection::<TestWorld>::new();
        
        execute(
            features,
            Some(1),
            collection,
            sender,
            None::<for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a mut TestWorld) -> futures::future::LocalBoxFuture<'a, ()>>,
            None::<for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a event::ScenarioFinished, Option<&'a mut TestWorld>) -> futures::future::LocalBoxFuture<'a, ()>>,
            false,
            #[cfg(feature = "tracing")]
            None,
            #[cfg(feature = "observability")]
            std::sync::Arc::new(std::sync::Mutex::new(crate::observer::ObserverRegistry::new())),
        ).await;
        
        // Should receive Started event
        let started = receiver.next().await.unwrap().unwrap();
        match started.value {
            event::Cucumber::Started => {},
            _ => panic!("Expected Started event"),
        }
        
        // Should receive Finished event
        let finished = receiver.next().await.unwrap().unwrap();
        match finished.value {
            event::Cucumber::Finished => {},
            _ => panic!("Expected Finished event"),
        }
        
        // No more events
        assert!(receiver.next().await.is_none());
    }

    #[test]
    fn test_scenario_type_determination() {
        let which_scenario = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, scenario: &gherkin::Scenario| {
            if scenario.tags.contains(&"@serial".to_string()) {
                ScenarioType::Serial
            } else {
                ScenarioType::Concurrent
            }
        };
        
        let feature = gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
        };
        
        let concurrent_scenario = gherkin::Scenario {
            tags: vec![],
            keyword: "Scenario".to_string(),
            name: "Concurrent".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            description: None,
            steps: vec![],
            examples: vec![],
        };
        
        let serial_scenario = gherkin::Scenario {
            tags: vec!["@serial".to_string()],
            keyword: "Scenario".to_string(),
            name: "Serial".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            description: None,
            steps: vec![],
            examples: vec![],
        };
        
        assert_eq!(which_scenario(&feature, None, &concurrent_scenario), ScenarioType::Concurrent);
        assert_eq!(which_scenario(&feature, None, &serial_scenario), ScenarioType::Serial);
    }
}