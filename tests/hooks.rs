//! Phase-end behavior acceptance for composable scenario hooks.
use std::sync::{Arc, Mutex};

use cucumber::{
    StatsWriter as _, World as _, WriterExt as _,
    event::ScenarioFinished,
    gherkin::{Feature, Rule, Scenario},
    given,
    runner::Hooks,
    then, when,
    writer::{self, Coloring},
};
use futures::future::LocalBoxFuture;

#[derive(Debug, Default)]
struct Observed {
    trace: Vec<String>,
    initial_sizes: Vec<usize>,
    attempts: usize,
    saw_failure: bool,
}

#[derive(Debug, Default, cucumber::World)]
struct Application {
    shared: Option<Arc<Mutex<Observed>>>,
    local: Vec<String>,
    config: String,
}

#[given("background work")]
fn background(world: &mut Application) {
    world
        .shared
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .trace
        .push("background".into());
}

#[when("application work")]
fn application_work(world: &mut Application) {
    let mut shared = world.shared.as_ref().unwrap().lock().unwrap();
    shared.trace.push("step".into());
    shared.attempts += 1;
    let fail = world.config == "step-failure"
        || (world.config == "retry" && shared.attempts == 1);
    drop(shared);
    assert!(!fail, "application failure");
}

fn before(
    name: &'static str,
    shared: Arc<Mutex<Observed>>,
    config: String,
) -> impl for<'a> Fn(
    &'a Feature,
    Option<&'a Rule>,
    &'a Scenario,
    &'a mut Application,
) -> LocalBoxFuture<'a, ()> {
    move |_, _, _, world| {
        let shared = Arc::clone(&shared);
        let config = config.clone();
        Box::pin(async move {
            world.shared = Some(Arc::clone(&shared));
            world.config = config.clone();
            {
                let mut observed = shared.lock().unwrap();
                observed.trace.push(name.into());
                if name == "before-one" {
                    observed.initial_sizes.push(world.local.len());
                }
            }
            world.local.push(name.into());
            assert!(
                config != "before-failure" || name != "before-one",
                "setup failure"
            );
        })
    }
}

fn after(
    name: &'static str,
    shared: Arc<Mutex<Observed>>,
    config: String,
) -> impl for<'a> Fn(
    &'a Feature,
    Option<&'a Rule>,
    &'a Scenario,
    &'a ScenarioFinished,
    Option<&'a mut Application>,
) -> LocalBoxFuture<'a, ()> {
    move |_, _, _, finished, world| {
        let shared = Arc::clone(&shared);
        let config = config.clone();
        Box::pin(async move {
            assert!(world.is_some(), "set up world reaches teardown");
            {
                let mut observed = shared.lock().unwrap();
                observed.trace.push(name.into());
                observed.saw_failure |=
                    matches!(finished, ScenarioFinished::StepFailed(..));
            }
            assert!(config != "after-failures", "cleanup failure");
        })
    }
}

#[derive(Debug, Default, cucumber::World)]
struct Lifecycle {
    config: String,
    observed: Arc<Mutex<Observed>>,
    failed_steps: usize,
    hook_errors: usize,
    output: String,
}

#[given(regex = r#"^the lifecycle configuration "([^"]+)"$"#)]
fn configuration(world: &mut Lifecycle, config: String) {
    world.config = config;
}

#[when("the application feature runs")]
async fn execute(world: &mut Lifecycle) {
    let config = world.config.clone();
    let shared = Arc::clone(&world.observed);
    let mut hooks = Hooks::new()
        .before(
            "before-one",
            before("before-one", Arc::clone(&shared), config.clone()),
        )
        .before(
            "before-two",
            before("before-two", Arc::clone(&shared), config.clone()),
        )
        .after(
            "after-one",
            after("after-one", Arc::clone(&shared), config.clone()),
        )
        .after(
            "after-two",
            after("after-two", Arc::clone(&shared), config.clone()),
        );
    let mut source = "Feature: Application\n  Background: Setup\n    Given \
        background work\n  Scenario: Request\n    When application work\n"
        .to_owned();
    if config == "sync-after-failure" {
        let shared = Arc::clone(&shared);
        hooks = hooks.after("after-sync", move |_, _, _, _, _| {
            shared.lock().unwrap().trace.push("after-sync".into());
            panic!("synchronous callback construction failure");
        });
    }
    if config == "tagged" {
        hooks = Hooks::new()
            .before_if(
                "excluded-before",
                "@absent".parse().unwrap(),
                before("excluded-before", Arc::clone(&shared), config.clone()),
            )
            .before_if(
                "tagged-before",
                "@feature and @rule and @scenario".parse().unwrap(),
                before("tagged-before", Arc::clone(&shared), config.clone()),
            )
            .after_if(
                "tagged-after",
                "@feature and @rule and @scenario".parse().unwrap(),
                after("tagged-after", Arc::clone(&shared), config.clone()),
            )
            .after_if(
                "excluded-after",
                "not @feature".parse().unwrap(),
                after("excluded-after", Arc::clone(&shared), config.clone()),
            );
        source = "@feature\nFeature: Application\n  Background: Setup\n    \
            Given background work\n  @rule\n  Rule: Account\n    \
            @scenario\n    Scenario: Request\n      When application work\n"
            .to_owned();
    }
    if config == "after-only" {
        let shared = Arc::clone(&shared);
        hooks = Hooks::new().after(
            "after-without-world",
            move |_, _, _, _, world| {
                let shared = Arc::clone(&shared);
                Box::pin(async move {
                    assert!(world.is_none());
                    shared
                        .lock()
                        .unwrap()
                        .trace
                        .push("after-without-world".into());
                })
            },
        );
        source = "Feature: Application\n  Scenario: No matching step\n    \
            Given undefined work\n"
            .to_owned();
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("application.feature");
    std::fs::write(&path, source).unwrap();
    let mut output = Vec::new();
    let runner = Application::cucumber()
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .max_concurrent_scenarios(1)
        .retries(if config == "retry" { 1 } else { 0 })
        .with_default_cli();
    let report = if config == "legacy" {
        runner
            .before(before("before-one", Arc::clone(&shared), config.clone()))
            .before(before("before-two", Arc::clone(&shared), config.clone()))
            .after(after("after-one", Arc::clone(&shared), config.clone()))
            .after(after("after-two", Arc::clone(&shared), config.clone()))
            .run(&path)
            .await
    } else {
        runner.with_hooks(hooks.clone()).run(&path).await
    };
    assert_eq!(report.parsing_errors(), 0);
    world.failed_steps = report.failed_steps();
    world.hook_errors = report.hook_errors();
    drop(report);
    world.output = String::from_utf8(output).unwrap();
}

#[then(regex = r#"^the lifecycle trace is "([^"]+)"$"#)]
fn trace(world: &mut Lifecycle, expected: String) {
    assert_eq!(
        world.observed.lock().unwrap().trace.join(","),
        expected,
        "{}",
        world.output
    );
}

#[then(regex = r"^the run has (\d+) failed steps and (\d+) hook errors$")]
fn result(world: &mut Lifecycle, steps: usize, hooks: usize) {
    assert_eq!(
        (world.failed_steps, world.hook_errors),
        (steps, hooks),
        "{}",
        world.output
    );
}

#[then(regex = r#"^the hook diagnostic contains "([^"]+)"$"#)]
fn diagnostic(world: &mut Lifecycle, name: String) {
    assert!(world.output.contains(&format!("`{name}`")), "{}", world.output);
}

#[then("teardown observes a failed step")]
fn failure_observed(world: &mut Lifecycle) {
    assert!(world.observed.lock().unwrap().saw_failure);
}

#[then("both attempts start with fresh world state")]
fn fresh_world(world: &mut Lifecycle) {
    assert_eq!(world.observed.lock().unwrap().initial_sizes, vec![0, 0]);
}

#[tokio::main]
async fn main() {
    Lifecycle::cucumber()
        .fail_on_skipped()
        .run_and_exit("tests/features/hooks")
        .await;
}
