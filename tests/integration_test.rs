use async_trait::async_trait;
use cucumber_rust::{event::*, t, Cucumber, EventHandler, Steps, World};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default, Clone)]
struct CustomEventHandler {
    state: Arc<Mutex<CustomEventHandlerState>>,
}
#[derive(Default)]
struct CustomEventHandlerState {
    any_rule_failures: bool,
    any_scenario_skipped: bool,
    any_scenario_failures: bool,
    any_step_unimplemented: bool,
    any_step_failures: bool,
    any_step_success: bool,
    any_step_timeouts: bool,
}
impl EventHandler for CustomEventHandler {
    fn handle_event(&mut self, event: CucumberEvent) {
        let mut state = self.state.lock().unwrap();
        match event {
            CucumberEvent::Feature(_feature, FeatureEvent::Rule(_rule, RuleEvent::Failed)) => {
                state.any_rule_failures = true;
            }
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Scenario(_scenario, ScenarioEvent::Failed),
            ) => {
                state.any_scenario_failures = true;
            }
            CucumberEvent::Feature(
                ref _feature,
                FeatureEvent::Scenario(ref _scenario, ScenarioEvent::Skipped),
            ) => {
                state.any_scenario_skipped = true;
            }
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Scenario(
                    _scenario,
                    ScenarioEvent::Step(_step, StepEvent::Failed(_, _)),
                ),
            ) => {
                state.any_step_failures = true;
            }
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Scenario(_scenario, ScenarioEvent::Step(_step, StepEvent::TimedOut)),
            ) => {
                state.any_step_timeouts = true;
            }
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Scenario(
                    _scenario,
                    ScenarioEvent::Step(_step, StepEvent::Unimplemented),
                ),
            ) => {
                state.any_step_unimplemented = true;
            }
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Scenario(_scenario, ScenarioEvent::Step(_step, StepEvent::Passed(_))),
            ) => {
                state.any_step_success = true;
            }
            _ => {}
        }
    }
}

#[derive(Default)]
struct StatelessWorld;

#[async_trait(?Send)]
impl World for StatelessWorld {
    type Error = std::convert::Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(StatelessWorld::default())
    }
}

#[test]
fn user_defined_event_handlers_are_expressible() {
    let custom_handler = CustomEventHandler::default();
    let mut steps = Steps::<StatelessWorld>::new();
    steps.when("something", |world, _step| world);
    steps.when("another thing", |world, _step| world);
    steps.then("it's okay", |world, _step| world);
    steps.then("it's not okay", |_world, _step| {
        panic!("Intentionally panicking to fail the step")
    });
    steps.then_async(
        "it takes a long time",
        t!(|world, _step| {
            futures_timer::Delay::new(Duration::from_secs(9_000)).await;
            world
        }),
    );

    let runner = Cucumber::with_handler(custom_handler.clone())
        .steps(steps)
        .features(&["./features/integration"])
        .step_timeout(Duration::from_secs(1));

    futures::executor::block_on(runner.run());

    let handler_state = custom_handler.state.lock().unwrap();
    assert!(!handler_state.any_rule_failures);
    assert!(handler_state.any_step_failures);
    assert!(handler_state.any_step_unimplemented);
    assert!(handler_state.any_step_success);
    assert!(handler_state.any_scenario_skipped);
    assert!(handler_state.any_step_timeouts);
}
