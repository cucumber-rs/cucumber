//! Core Executor struct and main scenario execution logic.

use std::sync::Arc;

use futures::{
    channel::mpsc,
    future::LocalBoxFuture,
};

#[cfg(feature = "tracing")]
use crate::tracing::SpanCloseWaiter;
use crate::{
    Event, World,
    event::{self, HookType, Info, Retries, source::Source},
    parser, step,
};

use super::super::{
    cli_and_types::{RetryOptions, ScenarioType},
    scenario_storage::{Features, FinishedFeaturesSender},
    supporting_structures::{
        ScenarioId, ExecutionFailure, AfterHookEventsMeta, IsFailed, IsRetried,
        coerce_into_info,
    },
};

use super::{
    events::EventSender,
    hooks::HookExecutor,
    steps::StepExecutor,
};

/// Runs [`Scenario`]s and notifies about their state of completion.
///
/// [`Scenario`]: gherkin::Scenario
pub struct Executor<W, Before, After> {
    /// [`Step`]s [`Collection`].
    ///
    /// [`Collection`]: step::Collection
    collection: step::Collection<W>,

    /// Function, executed on each [`Scenario`] before running all [`Step`]s,
    /// including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    before_hook: Option<Before>,

    /// Function, executed on each [`Scenario`] after running all [`Step`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    after_hook: Option<After>,

    /// Event sender for scenario events.
    event_sender: EventSender<W>,

    /// Sender for notifying of [`Scenario`]s completion.
    ///
    /// [`Scenario`]: gherkin::Scenario
    finished_sender: FinishedFeaturesSender,

    /// [`Scenario`]s storage.
    ///
    /// [`Scenario`]: gherkin::Scenario
    storage: Features,
}

impl<W: World, Before, After> Executor<W, Before, After>
where
    Before: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> LocalBoxFuture<'a, ()>,
    After: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>,
{
    /// Creates a new [`Executor`].
    pub fn new(
        collection: step::Collection<W>,
        before_hook: Option<Before>,
        after_hook: Option<After>,
        event_sender: mpsc::UnboundedSender<
            parser::Result<Event<event::Cucumber<W>>>,
        >,
        finished_sender: FinishedFeaturesSender,
        storage: Features,
    ) -> Self {
        Self {
            collection,
            before_hook,
            after_hook,
            event_sender: EventSender::new_with_sender(event_sender),
            finished_sender,
            storage,
        }
    }

    /// Runs a [`Scenario`] with the given [`ScenarioId`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub async fn run_scenario(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        retry_options: Option<RetryOptions>,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) {
        let retries = retry_options.map(|opts| opts.retries);

        // Create world instance for this scenario
        let mut world = match W::new().await {
            Ok(world) => world,
            Err(_err) => {
                // Emit world creation error as a before hook failure
                let error_info = coerce_into_info("Failed to create World");
                let started_event = event::Cucumber::scenario(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    event::RetryableScenario {
                        event: event::Scenario::Hook(
                            HookType::Before,
                            event::Hook::Failed(None, error_info.clone())
                        ),
                        retries,
                    },
                );
                self.event_sender.send_event(started_event);
                
                let finished_event = event::Cucumber::scenario(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    event::RetryableScenario {
                        event: event::Scenario::Finished,
                        retries,
                    },
                );
                self.event_sender.send_event(finished_event);
                
                // Check if scenario will be retried
                let next_try = retry_options
                    .and_then(RetryOptions::next_try);
                
                if let Some(next_try) = next_try {
                    self.storage
                        .insert_retried_scenario(
                            feature.clone(),
                            rule.clone(),
                            scenario,
                            scenario_ty,
                            Some(next_try),
                        )
                        .await;
                }
                
                self.scenario_finished(
                    id,
                    feature,
                    rule,
                    true, // World creation failure is a failure
                    next_try.is_some(),
                );
                return;
            }
        };

        // Send started event
        let started_event = event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::RetryableScenario {
                event: event::Scenario::Started,
                retries,
            },
        );
        self.event_sender.send_event(started_event);

        // Execute the scenario
        let execution_result = self
            .execute_scenario_steps(
                id,
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                &mut world,
                retries,
                #[cfg(feature = "tracing")]
                waiter,
            )
            .await;

        // Handle the scenario completion
        self.handle_scenario_completion(
            id,
            feature,
            rule,
            scenario,
            scenario_ty,
            execution_result,
            world,
            retry_options,
            #[cfg(feature = "tracing")]
            waiter,
        )
        .await;
    }

    /// Executes all steps of a scenario including hooks.
    async fn execute_scenario_steps(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        world: &mut W,
        retries: Option<Retries>,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) -> Result<AfterHookEventsMeta, ExecutionFailure<W>> {
        // Run before hook
        HookExecutor::run_before_hook(
            self.before_hook.as_ref(),
            id,
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            world,
            |event| self.event_sender.send_event(event),
            #[cfg(feature = "tracing")]
            waiter,
        )
        .await?;

        // Execute steps
        let step_results = StepExecutor::run_steps(
            &self.collection,
            id,
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            world,
            retries,
            |event| self.event_sender.send_event(event),
            #[cfg(feature = "tracing")]
            waiter,
        )
        .await;

        Ok(step_results)
    }

    /// Handles scenario completion and after hooks.
    async fn handle_scenario_completion(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        step_results: Result<AfterHookEventsMeta, ExecutionFailure<W>>,
        mut world: W,
        retry_options: Option<RetryOptions>,
        #[cfg(feature = "tracing")] waiter: Option<&SpanCloseWaiter>,
    ) {
        let retries = retry_options.map(|opts| opts.retries);
        // Check if this is actually a retry attempt (current > 0)
        let is_retry = retries.as_ref().is_some_and(|r| r.current > 0);
        
        let (meta, scenario_finished, is_failed) = match step_results {
            Ok(meta) => {
                let finished = meta.scenario_finished.clone();
                let failed = matches!(finished, event::ScenarioFinished::StepFailed(_, _, _));
                (meta, finished, failed)
            },
            Err(failure) => {
                let finished = failure.get_scenario_finished_event();
                let failed = true; // ExecutionFailure always indicates failure
                // Handle execution failure
                self.handle_execution_failure(
                    failure,
                    id,
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    retries,
                )
                .await;
                
                // Check if scenario will be retried
                let next_try = retry_options
                    .filter(|_| failed)
                    .and_then(RetryOptions::next_try);
                
                if let Some(next_try) = next_try {
                    // Insert scenario back into storage for retry
                    self.storage
                        .insert_retried_scenario(
                            feature.clone(),
                            rule.clone(),
                            scenario.clone(),
                            scenario_ty,
                            Some(next_try),
                        )
                        .await;
                }
                
                // Notify scenario finished
                self.scenario_finished(
                    id,
                    feature,
                    rule,
                    failed,
                    next_try.is_some(),
                );
                return;
            }
        };

        // Run after hook
        let after_hook_error = HookExecutor::run_after_hook(
            self.after_hook.as_ref(),
            id,
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            Some(&mut world),
            &scenario_finished,
            |event| self.event_sender.send_event(event),
            #[cfg(feature = "tracing")]
            waiter,
        )
        .await;

        let is_failed = is_failed || after_hook_error.is_some();

        // Send finished event
        let finished_event = event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::RetryableScenario {
                event: event::Scenario::Finished,
                retries,
            },
        );
        self.event_sender.send_event(finished_event);

        // Check if scenario will be retried
        let next_try = retry_options
            .filter(|_| is_failed)
            .and_then(RetryOptions::next_try);
        
        if let Some(next_try) = next_try {
            // Insert scenario back into storage for retry
            self.storage
                .insert_retried_scenario(
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    scenario_ty,
                    Some(next_try),
                )
                .await;
        }

        // Notify scenario finished (use next_try.is_some() to indicate if it will be retried)
        self.scenario_finished(
            id,
            feature,
            rule,
            is_failed,
            next_try.is_some(),
        );
    }

    /// Handles execution failures during scenario execution.
    /// 
    /// Note: The actual failure events are already emitted by the respective
    /// modules (hooks, steps) where the failures occur. This method is kept
    /// for potential future use but currently just sends the finished event.
    async fn handle_execution_failure(
        &self,
        _failure: ExecutionFailure<W>,
        _id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        retries: Option<Retries>,
    ) {
        // Failure events are already emitted by the respective modules
        // (hooks module for hook failures, steps module for step failures)
        // This method just sends the finished event
        
        let failure_event = event::Cucumber::scenario(
            feature,
            rule,
            scenario,
            event::RetryableScenario {
                event: event::Scenario::Finished,
                retries,
            },
        );
        self.event_sender.send_event(failure_event);
    }

    /// Sends a single event.
    pub fn send_event(&self, event: event::Cucumber<W>) {
        self.event_sender.send_event(event);
    }

    /// Sends multiple events.
    pub fn send_all_events(
        &self,
        events: impl IntoIterator<Item = event::Cucumber<W>>,
    ) {
        self.event_sender.send_all_events(events);
    }

    /// Notifies that a scenario has finished.
    fn scenario_finished(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        is_failed: IsFailed,
        is_retried: IsRetried,
    ) {
        // If the receiver end is dropped, then no one listens for events
        // so we can just ignore it.
        drop(
            self.finished_sender
                .unbounded_send((id, feature, rule, is_failed, is_retried)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::common::TestWorld;
    use futures::TryStreamExt as _;

    type BeforeHook = for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a mut TestWorld) -> LocalBoxFuture<'a, ()>;
    type AfterHook = for<'a> fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a event::ScenarioFinished, Option<&'a mut TestWorld>) -> LocalBoxFuture<'a, ()>;

    #[test]
    fn test_executor_creation() {
        let (_executor, _receiver) = create_test_executor();
        
        // Verify executor is created successfully
        assert!(true); // Basic creation test
    }

    fn create_test_executor() -> (Executor<TestWorld, BeforeHook, AfterHook>, mpsc::UnboundedReceiver<parser::Result<Event<event::Cucumber<TestWorld>>>>) {
        let collection = step::Collection::<TestWorld>::new();
        let (event_sender, event_receiver) = mpsc::unbounded();
        let (finished_sender, _finished_receiver) = mpsc::unbounded();
        let storage = Features::default();
        
        let executor = Executor::new(
            collection,
            None,
            None,
            event_sender,
            finished_sender,
            storage,
        );
        
        (executor, event_receiver)
    }

    fn create_test_feature_and_scenario() -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        use gherkin::{Feature, Scenario};
        
        let feature = Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span {
                start: 0,
                end: 0,
            },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };
        
        let scenario = Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span {
                start: 0,
                end: 0,
            },
            position: gherkin::LineCol { line: 2, col: 1 },
        };
        
        (Source::new(feature), Source::new(scenario))
    }
}