//! Phase 20 BDD for actual ordered per-step lifecycle behavior.
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use cucumber::{
    StatsWriter as _, World as _, WriterExt as _, given,
    runner::{Hooks, StepBodyOutcome, StepHookScope, StepHookStore, StepHooks},
    step::{Collection, Context, Matching},
    then, when,
    writer::{self, Coloring},
};
use futures::future::LocalBoxFuture;
use regex::Regex;
#[derive(Debug, Default)]
struct Seen {
    trace: Vec<String>,
    body: Vec<String>,
    initial: Vec<usize>,
}
#[derive(Debug, Default, cucumber::World)]
struct Application {
    seen: Arc<Mutex<Seen>>,
    config: String,
    local: usize,
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    config: String,
    seen: Arc<Mutex<Seen>>,
    store: StepHookStore,
    failed: usize,
    skipped: usize,
    output: String,
    path: std::path::PathBuf,
}
type HookFuture<'a> = LocalBoxFuture<'a, ()>;
fn before(
    name: &'static str,
    seen: Arc<Mutex<Seen>>,
    config: String,
) -> impl for<'a> Fn(&'a StepHookScope, &'a mut Application) -> HookFuture<'a> {
    move |scope, world| {
        let seen = seen.clone();
        let config = config.clone();
        Box::pin(async move {
            {
                let mut state = seen.lock().unwrap();
                state.trace.push(name.into());
                if name == "before-one" {
                    state.initial.push(world.local);
                }
            }
            world.local += 1;
            assert!(
                !(config == "before fails" && name == "before-one")
                    && config != "all fail"
                    && !(config == "retry"
                        && name == "before-one"
                        && scope.attempt == 0),
                "setup failure in {name}"
            );
        })
    }
}
fn after(
    name: &'static str,
    seen: Arc<Mutex<Seen>>,
    config: String,
) -> impl for<'a> Fn(
    &'a StepHookScope,
    &'a StepBodyOutcome,
    &'a mut Application,
) -> LocalBoxFuture<'a, ()> {
    move |_, body, world| {
        let seen = seen.clone();
        let config = config.clone();
        Box::pin(async move {
            assert!(
                world.local >= 2,
                "all before callbacks share the same World"
            );
            let label = match body {
                StepBodyOutcome::Pending | StepBodyOutcome::Skipped => panic!(
                    "legacy step-hook scenarios must not produce controlled \
                        completions"
                ),
                StepBodyOutcome::NotRun => "not run",
                StepBodyOutcome::Passed => "passed",
                StepBodyOutcome::Failed(message) => {
                    assert!(message.contains("body failure"));
                    "failed"
                }
            };
            {
                let mut state = seen.lock().unwrap();
                state.trace.push(name.into());
                state.body.push(label.into());
            }
            assert!(
                !(config == "after fails" && name == "after-two")
                    && config != "all fail",
                "cleanup failure in {name}"
            );
        })
    }
}
fn body(world: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        assert!(world.local >= 2, "setup reaches body");
        world.seen.lock().unwrap().trace.push("body".into());
        assert!(
            !matches!(world.config.as_str(), "body fails" | "later skipped"),
            "body failure"
        );
    })
}
#[given(expr = "a step lifecycle configured for {string}")]
fn configured(world: &mut Acceptance, config: String) {
    world.config = config;
}
#[given("a tagged feature and Rule with a background and scenario step")]
fn tagged(world: &mut Acceptance) {
    world.config = "tagged".into();
}
#[when("the native scenario executes")]
async fn execute(world: &mut Acceptance) {
    let config = world.config.clone();
    let seen = world.seen.clone();
    let mut hooks = StepHooks::new()
        .with_store(world.store.clone())
        .before(
            "before-one",
            before("before-one", seen.clone(), config.clone()),
        )
        .before_if(
            "before-two",
            if config == "tagged" {
                "@feature and @rule and @scenario"
            } else {
                "not @excluded"
            }
            .parse()
            .unwrap(),
            before("before-two", seen.clone(), config.clone()),
        )
        .after("after-one", after("after-one", seen.clone(), config.clone()))
        .after_if(
            "after-two",
            if config == "tagged" {
                "@feature and @rule and @scenario"
            } else {
                "not @excluded"
            }
            .parse()
            .unwrap(),
            after("after-two", seen.clone(), config.clone()),
        );
    let mut source =
        "Feature: Step lifecycle\n  Scenario: Request\n    Given work\n"
            .to_owned();
    if config == "undefined" {
        source = source.replace("Given work", "Given unknown");
    }
    if config == "later skipped" {
        source.push_str("    Given later\n");
    }
    if config == "tagged" {
        hooks = hooks
            .before_if(
                "excluded-before",
                "@absent".parse().unwrap(),
                before("excluded-before", seen.clone(), config.clone()),
            )
            .after_if(
                "excluded-after",
                "@absent".parse().unwrap(),
                after("excluded-after", seen.clone(), config.clone()),
            );
        source = "\
            @feature\n\
            Feature: Step lifecycle\n\
            \x20 Background: shared setup\n\
            \x20   Given work\n\
            \x20 @rule\n\
            \x20 Rule: Policy\n\
            \x20   @scenario\n\
            \x20   Scenario: Request\n\
            \x20     Given work\n"
            .into();
    }
    let mut steps = Collection::new().given(
        None,
        Regex::new("^(work|later)$").unwrap(),
        body,
    );
    if config == "ambiguous" {
        steps = steps.when(None, Regex::new("^work$").unwrap(), body);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("steps.feature");
    std::fs::write(&path, source).unwrap();
    world.path = path.canonicalize().unwrap();
    let mut output = Vec::new();
    // Replacing scenario hooks must retain installed step callbacks.
    let report = Application::cucumber()
        .steps(steps)
        .step_matching(Matching::KeywordIndependent)
        .with_step_hooks(hooks.clone())
        .with_hooks(
            Hooks::new()
                .before("replaced setup", |_, _, _, _| Box::pin(async {})),
        )
        .before(move |_, _, _, world| {
            world.seen = seen.clone();
            world.config = config.clone();
            Box::pin(async {})
        })
        .after(|_, _, _, _, _| Box::pin(async {}))
        .max_concurrent_scenarios(1)
        .retries(if world.config == "retry" { 1 } else { 0 })
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .run(&path)
        .await;
    assert_eq!(report.parsing_errors(), 0);
    assert_eq!(
        report.hook_errors(),
        0,
        "step hooks report native step failures"
    );
    world.failed = report.failed_steps();
    world.skipped = report.skipped_steps();
    drop(report);
    world.output = String::from_utf8(output).unwrap();
}
#[then(expr = "the callback and body trace is {string}")]
fn trace(world: &mut Acceptance, expected: String) {
    assert_eq!(
        world.seen.lock().unwrap().trace.join(","),
        expected,
        "{}",
        world.output
    );
}
#[then(expr = "the after-step callbacks observe {string}")]
fn body_result(world: &mut Acceptance, expected: String) {
    assert_eq!(
        world.seen.lock().unwrap().body,
        vec![expected.clone(), expected]
    );
}
#[then(expr = "the final native failed step count is {int}")]
fn failures(world: &mut Acceptance, count: usize) {
    assert_eq!(world.failed, count, "{}", world.output);
}
#[then("each actual hook invocation has its own identity, scope and outcome")]
fn records(world: &mut Acceptance) {
    let records = world.store.records();
    let trace = world.seen.lock().unwrap().trace.clone();
    assert_eq!(
        records.len(),
        trace.iter().filter(|entry| *entry != "body").count()
    );
    let mut ids = HashSet::new();
    let mut definitions = HashMap::new();
    for record in &records {
        assert!(ids.insert(record.invocation_id));
        assert!(record.finished_at >= record.started_at);
        assert_eq!(record.scope.feature.path.as_ref(), Some(&world.path));
        assert_eq!(record.scope.scenario.name, "Request");
        assert_eq!(record.scope.step.value, "work");
        assert!(record.scope.scenario.position.line > 0);
        assert!(record.scope.step.position.line > 0);
        if let Some(id) = definitions
            .insert(record.definition.name.clone(), record.definition.id)
        {
            assert_eq!(id, record.definition.id);
        }
        let expected_failure = match world.config.as_str() {
            "before fails" => record.definition.name == "before-one",
            "after fails" => record.definition.name == "after-two",
            "all fail" => true,
            "retry" => {
                record.definition.name == "before-one"
                    && record.scope.attempt == 0
            }
            _ => false,
        };
        assert_eq!(record.result.is_err(), expected_failure, "{record:?}");
        if let Err(error) = &record.result {
            assert!(error.contains("failure"));
        }
    }
    assert_eq!(definitions.len(), 4);
}
#[then("no step hook or body invocation occurs")]
fn none(world: &mut Acceptance) {
    assert!(world.seen.lock().unwrap().trace.is_empty());
    assert!(world.store.records().is_empty());
}
#[then("the native scenario reports one skipped step")]
fn skipped(world: &mut Acceptance) {
    assert_eq!(world.skipped, 1);
}
#[then("selected hooks surround both steps and excluded hooks never run")]
fn selected(world: &mut Acceptance) {
    assert_eq!(
        world.seen.lock().unwrap().trace.join(","),
        "before-one,before-two,body,after-two,after-one,before-one,before-two\
            ,body,after-two,after-one"
    );
    let records = world.store.records();
    assert_eq!(records.iter().filter(|r| r.scope.is_background).count(), 4);
    assert_eq!(records.iter().filter(|r| !r.scope.is_background).count(), 4);
    for record in records {
        assert_eq!(record.scope.rule.as_ref().unwrap().name, "Policy");
        assert!(!record.definition.name.contains("excluded"));
    }
}
#[then(
    "both attempts have complete hook traces and distinct execution \
        identities"
)]
fn attempts(world: &mut Acceptance) {
    assert_eq!(
        world.seen.lock().unwrap().trace.join(","),
        "before-one,before-two,after-two,after-one,before-one,before-two,body\
            ,after-two,after-one"
    );
    assert_eq!(world.seen.lock().unwrap().initial, vec![0, 0]);
    let records = world.store.records();
    assert_eq!(records.len(), 8);
    let mut ids = HashSet::new();
    for attempt in 0..2 {
        let scoped = records
            .iter()
            .filter(|r| r.scope.attempt == attempt)
            .collect::<Vec<_>>();
        assert_eq!(scoped.len(), 4);
        let id = scoped[0].scope.scenario_id;
        assert!(scoped.iter().all(|r| r.scope.scenario_id == id));
        assert!(ids.insert(id));
    }
    assert_eq!(world.failed, 0);
}
#[then("only the first attempt has a failed hook record")]
fn retry_failure(world: &mut Acceptance) {
    let failed = world
        .store
        .records()
        .into_iter()
        .filter(|r| r.result.is_err())
        .collect::<Vec<_>>();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].scope.attempt, 0);
}
#[then("hook records refer only to the first executed step")]
fn unexecuted(world: &mut Acceptance) {
    let records = world.store.records();
    assert_eq!(records.len(), 4);
    assert!(records.iter().all(|r| r.scope.step.value == "work"));
    assert_eq!(
        world
            .seen
            .lock()
            .unwrap()
            .trace
            .iter()
            .filter(|s| s.as_str() == "body")
            .count(),
        1
    );
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .fail_on_skipped()
        .run_and_exit("tests/features/step_hooks")
        .await;
}
