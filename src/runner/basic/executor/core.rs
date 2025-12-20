//! Core Executor struct and main scenario execution logic.

use futures::{
    channel::mpsc,
    future::LocalBoxFuture,
};

#[cfg(feature = "tracing")]
use crate::tracing::SpanCloseWaiter;
use crate::{
    Event, World,
    event::{self, HookType, Retries, source::Source},
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
#[cfg(not(feature = "observability"))]
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

/// Runs [`Scenario`]s and notifies about their state of completion (with observability).
///
/// [`Scenario`]: gherkin::Scenario
#[cfg(feature = "observability")]
pub struct Executor<W: World, Before, After> {
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
    
    /// Observer registry for external monitoring
    observers: std::sync::Arc<std::sync::Mutex<crate::observer::ObserverRegistry<W>>>,
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
        #[cfg(feature = "observability")]
        observers: std::sync::Arc<std::sync::Mutex<crate::observer::ObserverRegistry<W>>>,
    ) -> Self {
        Self {
            collection,
            before_hook,
            after_hook,
            #[cfg(not(feature = "observability"))]
            event_sender: EventSender::new_with_sender(event_sender),
            #[cfg(feature = "observability")]
            event_sender: EventSender::with_observers(event_sender, observers.clone()),
            finished_sender,
            storage,
            #[cfg(feature = "observability")]
            observers,
        }
    }
    
    /// Register an observer for monitoring test execution
    #[cfg(feature = "observability")]
    pub fn register_observer(&self, observer: Box<dyn crate::observer::TestObserver<W>>) {
        if let Ok(mut registry) = self.observers.lock() {
            registry.register(observer);
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
        // Set the scenario context for observer notifications
        #[cfg(feature = "observability")]
        {
            let retries = retry_options.as_ref().map(|opts| opts.retries);
            self.event_sender.set_scenario_context(
                id,
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                retries,
            );
        }
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
        
        // Clear the scenario context after completion
        #[cfg(feature = "observability")]
        self.event_sender.clear_scenario_context();
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
        let _is_retry = retries.as_ref().is_some_and(|r| r.current > 0);
        
        let (_meta, scenario_finished, is_failed) = match step_results {
            Ok(meta) => {
                let finished = meta.scenario_finished.clone();
                let failed = matches!(finished, event::ScenarioFinished::StepFailed(_, _, _));
                (meta, finished, failed)
            },
            Err(failure) => {
                let _finished = failure.get_scenario_finished_event();
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
        // Send through normal channel
        let _event_wrapped = Event::new(event.clone());
        self.event_sender.send_event(event);
        
        // Notify observers if enabled
        #[cfg(feature = "observability")]
        {
            if let Ok(mut registry) = self.observers.lock() {
                // Build context from current event information
                let context = match &event {
                    event::Cucumber::Feature(feature_src, feature_event) => {
                        let feature_name = feature_src.name.clone();
                        
                        // Extract scenario, rule, and retry information from the event
                        let (scenario_name, rule_name, retry_info, tags) = match feature_event {
                            event::Feature::Scenario(scenario_src, retryable) => {
                                (
                                    scenario_src.name.clone(),
                                    None,
                                    Some(retryable.retries),
                                    scenario_src.tags.iter().map(|t| t.to_string()).collect(),
                                )
                            }
                            event::Feature::Rule(rule_src, rule_event) => {
                                match rule_event {
                                    event::Rule::Scenario(scenario_src, retryable) => {
                                        (
                                            scenario_src.name.clone(),
                                            Some(rule_src.name.clone()),
                                            Some(retryable.retries),
                                            scenario_src.tags.iter().map(|t| t.to_string()).collect(),
                                        )
                                    }
                                    _ => (String::new(), Some(rule_src.name.clone()), None, Vec::new()),
                                }
                            }
                            _ => (String::new(), None, None, Vec::new()),
                        };
                        
                        crate::observer::ObservationContext {
                            scenario_id: None, // ScenarioId is not available at this level
                            feature_name,
                            rule_name,
                            scenario_name,
                            retry_info,
                            tags,
                            timestamp: std::time::Instant::now(),
                        }
                    }
                    _ => {
                        // For non-feature events, provide minimal context
                        crate::observer::ObservationContext {
                            scenario_id: None,
                            feature_name: String::new(),
                            rule_name: None,
                            scenario_name: String::new(),
                            retry_info: None,
                            tags: Vec::new(),
                            timestamp: std::time::Instant::now(),
                        }
                    }
                };
                
                registry.notify(&event_wrapped, &context);
            }
        }
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

    #[test]
    fn test_send_event() {
        let (executor, mut receiver) = create_test_executor();
        let test_event = event::Cucumber::<TestWorld>::Started;
        
        executor.send_event(test_event);
        
        // Verify event was sent
        let received = receiver.try_next().unwrap();
        assert!(received.is_some());
        assert!(matches!(received.unwrap().unwrap().value, event::Cucumber::Started));
    }

    #[test]
    fn test_send_all_events() {
        let (executor, mut receiver) = create_test_executor();
        let events = vec![
            event::Cucumber::<TestWorld>::Started,
            event::Cucumber::Finished,
        ];
        
        executor.send_all_events(events);
        
        // Verify both events were sent
        let first = receiver.try_next().unwrap();
        assert!(first.is_some());
        let second = receiver.try_next().unwrap();
        assert!(second.is_some());
    }

    #[test]
    fn test_scenario_finished_notification() {
        let collection = step::Collection::<TestWorld>::new();
        let (event_sender, _event_receiver) = mpsc::unbounded();
        let (finished_sender, mut finished_receiver) = mpsc::unbounded();
        let storage = Features::default();
        
        let executor: Executor<TestWorld, BeforeHook, AfterHook> = Executor::new(
            collection,
            None,
            None,
            event_sender,
            finished_sender,
            storage,
            #[cfg(feature = "observability")]
            std::sync::Arc::new(std::sync::Mutex::new(crate::observer::ObserverRegistry::new())),
        );
        
        let id = ScenarioId::new();
        let (feature, _scenario) = create_test_feature_and_scenario();
        
        // Notify scenario finished
        executor.scenario_finished(
            id,
            feature.clone(),
            None, // No rule
            false, // Not failed
            false, // Not retried
        );
        
        // Verify notification was sent
        let notification = finished_receiver.try_next().unwrap();
        assert!(notification.is_some());
        let (received_id, received_feature, _rule, is_failed, is_retried) = notification.unwrap();
        assert_eq!(received_id, id);
        assert_eq!(received_feature.name, feature.name);
        assert!(!is_failed);
        assert!(!is_retried);
    }

    #[tokio::test]
    async fn test_handle_execution_failure() {
        let (executor, mut receiver) = create_test_executor();
        let (feature, scenario) = create_test_feature_and_scenario();
        let id = ScenarioId::new();
        
        let info = crate::runner::basic::supporting_structures::coerce_into_info("Before hook failed");
        let meta = event::Metadata::new(());
        let failure = ExecutionFailure::BeforeHookPanicked {
            world: None,
            panic_info: info,
            meta,
        };
        
        executor.handle_execution_failure(
            failure,
            id,
            feature.clone(),
            None, // No rule
            scenario.clone(),
            None, // No retries
        ).await;
        
        // Should send finished event
        let event = receiver.try_next().unwrap();
        assert!(event.is_some());
        match event.unwrap().unwrap().value {
            event::Cucumber::Feature(_, event::Feature::Scenario(_, event::RetryableScenario { 
                event: event::Scenario::Finished, 
                .. 
            })) => {},
            _ => panic!("Expected Scenario::Finished event"),
        }
    }

    #[cfg(feature = "observability")]
    #[test]
    fn test_register_observer() {
        use crate::observer::{TestObserver, ObservationContext};
        
        struct MockObserver;
        impl TestObserver<TestWorld> for MockObserver {
            fn on_event(&mut self, _event: &Event<event::Cucumber<TestWorld>>, _ctx: &ObservationContext) {}
        }
        
        let (executor, _) = create_test_executor();
        let observer = Box::new(MockObserver);
        
        // Should not panic
        executor.register_observer(observer);
    }

    fn create_test_executor() -> (Executor<TestWorld, BeforeHook, AfterHook>, mpsc::UnboundedReceiver<parser::Result<Event<event::Cucumber<TestWorld>>>>) {
        let collection = step::Collection::<TestWorld>::new();
        let (event_sender, event_receiver) = mpsc::unbounded();
        let (finished_sender, _finished_receiver) = mpsc::unbounded();
        let storage = Features::default();
        
        let executor: Executor<TestWorld, BeforeHook, AfterHook> = Executor::new(
            collection,
            None,
            None,
            event_sender,
            finished_sender,
            storage,
            #[cfg(feature = "observability")]
            std::sync::Arc::new(std::sync::Mutex::new(crate::observer::ObserverRegistry::new())),
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