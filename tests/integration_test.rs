use async_trait::async_trait;
use cucumber_rust::{event::*, t, Cucumber, EventHandler, Steps, World};
use serial_test::serial;
use std::path::PathBuf;
use std::process::Command;
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
    fn handle_event(&mut self, event: &CucumberEvent) {
        let mut state = self.state.lock().unwrap();
        match event {
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Rule(_rule, RuleEvent::Failed(FailureKind::Panic)),
            ) => {
                state.any_rule_failures = true;
            }
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Scenario(_scenario, ScenarioEvent::Failed(FailureKind::Panic)),
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
                    ScenarioEvent::Step(_step, StepEvent::Failed(StepFailureKind::Panic(_, _))),
                ),
            ) => {
                state.any_step_failures = true;
            }
            CucumberEvent::Feature(
                _feature,
                FeatureEvent::Scenario(
                    _scenario,
                    ScenarioEvent::Step(_step, StepEvent::Failed(StepFailureKind::TimedOut)),
                ),
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

fn stateless_steps() -> Steps<StatelessWorld> {
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
    steps
}

#[test]
#[serial]
fn user_defined_event_handlers_are_expressible() {
    let custom_handler = CustomEventHandler::default();

    let runner = Cucumber::with_handler(custom_handler.clone())
        .steps(stateless_steps())
        .features(&["./features/integration"])
        .step_timeout(Duration::from_secs(1));

    let results = futures::executor::block_on(runner.run());

    assert_eq!(results.features.total, 1);
    assert_eq!(results.scenarios.total, 4);
    assert_eq!(results.steps.total, 14);
    assert_eq!(results.steps.passed, 4);
    assert_eq!(results.scenarios.failed, 1);

    let handler_state = custom_handler.state.lock().unwrap();
    assert!(!handler_state.any_rule_failures);
    assert!(handler_state.any_step_failures);
    assert!(handler_state.any_step_unimplemented);
    assert!(handler_state.any_step_success);
    assert!(handler_state.any_scenario_skipped);
    assert!(handler_state.any_step_timeouts);
}

fn nocapture_enabled() -> bool {
    std::env::args_os().any(|a| {
        if let Some(s) = a.to_str() {
            s == "--nocapture"
        } else {
            false
        }
    }) || match std::env::var("RUST_TEST_NOCAPTURE") {
        Ok(val) => &val != "0",
        Err(_) => false,
    }
}

#[test]
#[serial]
fn enable_capture_false_support() {
    if !nocapture_enabled() {
        // This test only functions when the Rust test framework is refraining
        // from swallowing all output from this process (and child processes)
        // Execute with `cargo test -- --nocapture` to see the real results
        return;
    }
    let command_output = Command::new(built_executable_path("capture-runner"))
        .args(&["false"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Could not execute capture-runner");
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    assert!(stdout.contains("everything is great"));
    assert!(stderr.contains("something went wrong"));
    assert!(
        command_output.status.success(),
        "capture-runner should exit successfully"
    );
}
fn get_target_dir() -> PathBuf {
    let bin = std::env::current_exe().expect("exe path");
    let mut target_dir = PathBuf::from(bin.parent().expect("bin parent"));
    while target_dir.file_name() != Some(std::ffi::OsStr::new("target")) {
        target_dir.pop();
    }
    target_dir
}

fn built_executable_path(name: &str) -> PathBuf {
    let program_path =
        get_target_dir()
            .join("debug")
            .join(format!("{}{}", name, std::env::consts::EXE_SUFFIX));

    program_path.canonicalize().expect(&format!(
        "Cannot resolve {} at {:?}",
        name,
        program_path.display()
    ))
}
