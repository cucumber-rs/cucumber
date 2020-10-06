use async_trait::async_trait;
use cucumber_rust::{event::*, output::BasicOutput, Cucumber, EventHandler, Steps, World};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct CaptureRunnerWorld;

#[async_trait(?Send)]
impl World for CaptureRunnerWorld {
    type Error = std::convert::Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(CaptureRunnerWorld::default())
    }
}
/// Event handler that delegates for printing to the default event handler,
/// but also captures whether any steps failed, were skipped, timed out,
/// or were unimplemented.
#[derive(Clone, Default)]
pub struct ProblemDetectingEventHandler {
    pub state: Arc<Mutex<ProblemDetectingEventHandlerState>>,
}

#[derive(Default)]
pub struct ProblemDetectingEventHandlerState {
    pub basic_output: BasicOutput,
    pub any_problem: bool,
}

impl EventHandler for ProblemDetectingEventHandler {
    fn handle_event(&mut self, event: CucumberEvent) {
        let mut state = self.state.lock().unwrap();
        match &event {
            CucumberEvent::Feature(
                _,
                FeatureEvent::Scenario(_, ScenarioEvent::Step(_, StepEvent::Failed(_, _))),
            )
            | CucumberEvent::Feature(
                _,
                FeatureEvent::Scenario(_, ScenarioEvent::Step(_, StepEvent::Skipped)),
            )
            | CucumberEvent::Feature(
                _,
                FeatureEvent::Scenario(_, ScenarioEvent::Step(_, StepEvent::TimedOut)),
            )
            | CucumberEvent::Feature(
                _,
                FeatureEvent::Scenario(_, ScenarioEvent::Step(_, StepEvent::Unimplemented)),
            ) => {
                state.any_problem = true;
            }
            _ => {}
        }
        state.basic_output.handle_event(event);
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Requires a boolean argument [true | false] to indicate whether to enable output capture");
        std::process::exit(1);
    }
    let enable_capture: bool = args[1].parse().unwrap();
    let mut steps = Steps::<CaptureRunnerWorld>::new();

    steps.when(
        r#"we print "everything is great" to stdout"#,
        |world, _step| {
            println!("everything is great");
            world
        },
    );
    steps.when(
        r#"we print "something went wrong" to stderr"#,
        |world, _step| {
            eprintln!("something went wrong");
            world
        },
    );
    steps.then(
        "it is up to the cucumber configuration to decide whether the content gets printed",
        |world, _step| world,
    );

    let event_handler = ProblemDetectingEventHandler::default();
    let runner = Cucumber::with_handler(event_handler.clone())
        .steps(steps)
        .features(&["./features/capture"])
        .enable_capture(enable_capture);

    futures::executor::block_on(runner.run());
    let handler_state = event_handler.state.lock().unwrap();

    if handler_state.any_problem {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}
