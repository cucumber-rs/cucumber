//! Phase35 formats: dry-run classification flags reach actual built-in
//! consumers.
use std::{
    fmt, io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use cucumber::{
    StatsWriter as _, World as _, WriterExt as _,
    execution::{ExecutionMode, OutcomePolicy},
    given,
    runner::{Hooks, RunHooks, StepHooks},
    step::{Collection, Context, Matching},
    then, when,
    writer::{self, Coloring},
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;
static FAIL_CONSTRUCTOR: AtomicBool = AtomicBool::new(false);
static CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);
static BODIES: AtomicUsize = AtomicUsize::new(0);
static CALLBACKS: AtomicUsize = AtomicUsize::new(0);
#[derive(Clone, Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("captured report")
    }
}
impl io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Buffer {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}
#[derive(Debug)]
struct Application;
impl cucumber::World for Application {
    type Error = &'static str;
    async fn new() -> Result<Self, Self::Error> {
        CONSTRUCTED.fetch_add(1, Ordering::SeqCst);
        if FAIL_CONSTRUCTOR.load(Ordering::SeqCst) {
            Err("dry-run constructor rejected")
        } else {
            Ok(Self)
        }
    }
}
fn act(_: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {
        BODIES.fetch_add(1, Ordering::SeqCst);
    })
}
fn callback() {
    CALLBACKS.fetch_add(1, Ordering::SeqCst);
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    consumer: String,
    constructor_failure: bool,
    restored: bool,
    output: String,
    failed: bool,
    retried: usize,
    constructed: usize,
    bodies: usize,
    callbacks: usize,
}
async fn execute<Wr>(writer: Wr, fail: bool, restored: bool) -> (bool, usize)
where
    Wr: cucumber::Writer<Application>
        + writer::Stats<Application>
        + writer::Normalized,
    Wr::Cli: Default,
{
    FAIL_CONSTRUCTOR.store(fail, Ordering::SeqCst);
    CONSTRUCTED.store(0, Ordering::SeqCst);
    BODIES.store(0, Ordering::SeqCst);
    CALLBACKS.store(0, Ordering::SeqCst);
    let steps = Collection::new()
        .given(None, Regex::new("^known$").unwrap(), act)
        .given(None, Regex::new("^ambiguous$").unwrap(), act)
        .when(None, Regex::new("^ambig.*$").unwrap(), act);
    let hooks = Hooks::new()
        .before("setup", |_, _, _, _| {
            Box::pin(async {
                callback();
            })
        })
        .after("cleanup", |_, _, _, _, _| {
            Box::pin(async {
                callback();
            })
        });
    let step_hooks = StepHooks::new()
        .before("before step", |_, _| {
            Box::pin(async {
                callback();
            })
        })
        .after("after step", |_, _, _| {
            Box::pin(async {
                callback();
            })
        });
    let run_hooks = RunHooks::new()
        .before("before run", |_| {
            Box::pin(async {
                callback();
            })
        })
        .after("after run", |_| {
            Box::pin(async {
                callback();
            })
        });
    let source = if fail {
        "Feature: Constructor\n Scenario: initialization\n  Given known\n"
    } else {
        "Feature: Inspection\n Scenario: recognized\n  Given known\n \
            Scenario: undefined\n  Given missing\n Scenario: ambiguous\n  \
            Given ambiguous\n"
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dry.feature");
    std::fs::write(&path, source).unwrap();
    let mut run = cucumber::Cucumber::new()
        .steps(steps)
        .step_matching(Matching::KeywordIndependent)
        .outcome_policy(OutcomePolicy::cucumber_js())
        .execution_mode(ExecutionMode::DryRun)
        .with_hooks(hooks)
        .with_step_hooks(step_hooks)
        .with_writer(writer)
        .with_run_hooks(run_hooks)
        .retries(2)
        .max_concurrent_scenarios(2);
    if restored {
        run = run.execution_mode(ExecutionMode::Run);
    }
    let report = run.with_default_cli().run(&path).await;
    assert_eq!(report.parsing_errors(), 0);
    (report.execution_has_failed(), report.retried_steps())
}
#[given(expr = "a {string} report for recognized, undefined and ambiguous \
        definitions")]
fn selected(world: &mut Acceptance, consumer: String) {
    world.consumer = consumer;
}
#[given(expr = "a {string} report whose World constructor fails")]
fn constructor(world: &mut Acceptance, consumer: String) {
    world.consumer = consumer;
    world.constructor_failure = true;
}
async fn run(world: &mut Acceptance) {
    let output = Buffer::default();
    let (failed, retried) = match world.consumer.as_str() {
        "JSON" => {
            execute(
                writer::Json::new(output.clone())
                    .discard_arbitrary_writes()
                    .summarized(),
                world.constructor_failure,
                world.restored,
            )
            .await
        }
        "JUnit" => {
            let observed =
                writer::Basic::raw(Buffer::default(), Coloring::Never, 0)
                    .summarized()
                    .tee::<Application, _>(writer::JUnit::for_tee(
                        output.clone(),
                        0,
                    ))
                    .normalized();
            execute(observed, world.constructor_failure, world.restored).await
        }
        "libtest" => {
            execute(
                writer::Libtest::new(output.clone()),
                world.constructor_failure,
                world.restored,
            )
            .await
        }
        other => panic!("unknown consumer {other}"),
    };
    world.failed = failed;
    world.retried = retried;
    world.output = output.text();
    world.constructed = CONSTRUCTED.load(Ordering::SeqCst);
    world.bodies = BODIES.load(Ordering::SeqCst);
    world.callbacks = CALLBACKS.load(Ordering::SeqCst);
}
#[when("the application is inspected without executing callbacks")]
async fn inspect(world: &mut Acceptance) {
    run(world).await;
}
#[when("inspection is turned off and the application executes")]
async fn restore(world: &mut Acceptance) {
    world.restored = true;
    run(world).await;
}
fn libtest(world: &Acceptance) -> Vec<Value> {
    world
        .output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn assert_classifications(world: &Acceptance, executed: bool) {
    match world.consumer.as_str() {
        "JSON" => {
            let data: Value = serde_json::from_str(&world.output).unwrap();
            let elements = data[0]["elements"].as_array().unwrap();
            assert_eq!(elements.len(), 3);
            for (name, status) in [
                ("recognized", if executed { "passed" } else { "skipped" }),
                ("undefined", "undefined"),
                ("ambiguous", "ambiguous"),
            ] {
                let element =
                    elements.iter().find(|e| e["name"] == name).unwrap();
                assert_eq!(
                    element["steps"][0]["result"]["status"], status,
                    "{element}"
                );
            }
            let ambiguous =
                elements.iter().find(|e| e["name"] == "ambiguous").unwrap();
            let message = ambiguous["steps"][0]["result"]["error_message"]
                .as_str()
                .unwrap();
            assert!(message.contains("^ambiguous$"));
            assert!(message.contains("^ambig.*$"));
        }
        "JUnit" => {
            assert_eq!(
                world.output.matches("<testcase ").count(),
                3,
                "{}",
                world.output
            );
            assert_eq!(
                world.output.matches("<failure").count(),
                if executed { 2 } else { 0 }
            );
            assert_eq!(
                world.output.matches("<skipped").count(),
                if executed { 0 } else { 3 }
            );
            assert!(world.output.to_lowercase().contains("undefined"));
            assert!(world.output.contains("^ambiguous$"));
            assert!(world.output.contains("^ambig.*$"));
        }
        "libtest" => {
            let messages = libtest(world);
            let suite = messages
                .iter()
                .find(|m| m["type"] == "suite" && m["event"] != "started")
                .unwrap();
            assert_eq!(suite["event"], if executed { "failed" } else { "ok" });
            assert_eq!(suite["failed"], if executed { 2 } else { 0 });
            for (name, status) in [
                ("recognized", if executed { "ok" } else { "ignored" }),
                ("undefined", if executed { "failed" } else { "ignored" }),
                ("ambiguous", if executed { "failed" } else { "ignored" }),
            ] {
                let events: Vec<_> = messages
                    .iter()
                    .filter(|m| {
                        m["type"] == "test"
                            && m["name"]
                                .as_str()
                                .is_some_and(|n| n.contains(name))
                            && m["event"] != "started"
                    })
                    .collect();
                assert!(!events.is_empty());
                assert!(
                    events.iter().any(|m| m["event"] == status),
                    "{events:?}"
                );
            }
            assert!(world.output.contains("^ambiguous$"));
            assert!(world.output.contains("^ambig.*$"));
        }
        _ => unreachable!(),
    }
}
#[then("all classifications remain distinguishable and the report succeeds")]
fn succeeds(world: &mut Acceptance) {
    assert!(!world.failed, "{}", world.output);
    assert_classifications(world, false);
}
#[then("three Worlds are constructed once with no callbacks or retries")]
fn suppressed(world: &mut Acceptance) {
    assert_eq!(world.constructed, 3);
    assert_eq!((world.bodies, world.callbacks, world.retried), (0, 0, 0));
}
#[then("the report preserves the initialization failure and fails")]
fn initialization(world: &mut Acceptance) {
    assert!(world.failed, "{}", world.output);
    assert_eq!(
        (world.constructed, world.bodies, world.callbacks, world.retried),
        (1, 0, 0, 0)
    );
    assert!(world.output.contains("dry-run constructor rejected"));
    match world.consumer.as_str() {
        "JSON" => {
            let data: Value = serde_json::from_str(&world.output).unwrap();
            assert_eq!(
                data[0]["elements"][0]["before"][0]["result"]["status"],
                "failed"
            );
        }
        "JUnit" => assert_eq!(world.output.matches("<failure").count(), 1),
        "libtest" => {
            let messages = libtest(world);
            let suite = messages
                .iter()
                .find(|m| m["type"] == "suite" && m["event"] != "started")
                .unwrap();
            assert_eq!(suite["event"], "failed");
            assert_eq!(suite["failed"], 1);
        }
        _ => unreachable!(),
    }
}
#[then(
    "the recognized body passes while undefined and ambiguous cases fail the \
        report"
)]
fn executed(world: &mut Acceptance) {
    assert!(world.failed);
    assert_eq!(world.bodies, 1);
    assert!(world.callbacks > 0);
    assert_eq!(world.retried, 0);
    assert_classifications(world, true);
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/dry_run_formats")
        .await;
}
