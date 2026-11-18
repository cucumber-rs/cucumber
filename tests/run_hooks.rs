//! Phase24 BDD for native run-level setup/cleanup and process failure.
use cucumber::{
    StatsWriter as _, World as _, WriterExt as _, given,
    runner::{RunHookKind, RunHookScope, RunHookStore, RunHooks},
    step::{Collection, Context},
    then, when,
    writer::{self, Coloring},
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use std::{
    collections::HashSet,
    process::Command,
    sync::{Arc, Mutex},
};
#[derive(Debug, Default)]
struct Seen {
    trace: Vec<String>,
    bodies: usize,
    setup: usize,
    scopes: Vec<(u64, u64, u64)>,
}
#[derive(Debug, Default, cucumber::World)]
struct Application {
    seen: Arc<Mutex<Seen>>,
    config: String,
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    config: String,
    seen: Arc<Mutex<Seen>>,
    store: RunHookStore,
    hook_errors: usize,
    step_errors: usize,
    failed: bool,
    child_error: String,
    separate: bool,
}
fn callback(
    name: &'static str,
    seen: Arc<Mutex<Seen>>,
    config: String,
) -> impl for<'a> Fn(&'a RunHookScope) -> LocalBoxFuture<'a, ()> {
    move |scope| {
        if (config == "before construct fails" && name == "before-one")
            || (config == "after construct fails" && name == "after-two")
        {
            {
                let mut state = seen.lock().unwrap();
                state.trace.push(name.into());
                state
                    .scopes
                    .push((scope.run_id, scope.invocation_id, scope.definition.id));
                if name.starts_with("before") {
                    state.setup += 1;
                }
            }
            panic!("global callback construction failure in {name}");
        }
        let seen = seen.clone();
        let config = config.clone();
        Box::pin(async move {
            {
                let mut state = seen.lock().unwrap();
                state.trace.push(name.into());
                state
                    .scopes
                    .push((scope.run_id, scope.invocation_id, scope.definition.id));
                if name.starts_with("before") {
                    state.setup += 1;
                }
            }
            let fail = match config.as_str() {
                "before fails" => name == "before-one",
                "before both" => name.starts_with("before"),
                "after fails" => name == "after-two",
                "all fail" => true,
                _ => false,
            };
            assert!(!fail, "global callback failure in {name}");
        })
    }
}
fn hooks(world: &Acceptance) -> RunHooks {
    RunHooks::new()
        .with_store(world.store.clone())
        .before(
            "before-one",
            callback("before-one", world.seen.clone(), world.config.clone()),
        )
        .before(
            "before-two",
            callback("before-two", world.seen.clone(), world.config.clone()),
        )
        .after(
            "after-one",
            callback("after-one", world.seen.clone(), world.config.clone()),
        )
        .after(
            "after-two",
            callback("after-two", world.seen.clone(), world.config.clone()),
        )
}
fn body(world: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let count = {
            let mut state = world.seen.lock().unwrap();
            assert_eq!(state.setup, 2);
            state.trace.push("body".into());
            state.bodies += 1;
            state.bodies
        };
        assert!(
            world.config != "body fails" && !(world.config == "retry" && count == 1),
            "native body failure"
        );
    })
}
fn feature(kind: &str) -> String {
    if kind == "empty" {
        "Feature: Global lifecycle\n".into()
    } else if kind == "parallel" {
        "Feature: Global lifecycle\n  Scenario: First\n    Given work\n  Scenario: Second\n    Given work\n".into()
    } else {
        "Feature: Global lifecycle\n  Scenario: Work\n    Given work\n".into()
    }
}
#[given(expr = "a run lifecycle configured for {string}")]
fn configured(world: &mut Acceptance, config: String) {
    world.config = config;
}
#[when("the native run completes with global hooks")]
async fn execute(world: &mut Acceptance) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("run.feature");
    std::fs::write(&path, feature(&world.config)).unwrap();
    let hooks = hooks(world);
    let seen = world.seen.clone();
    let config = world.config.clone();
    let mut output = Vec::new();
    let result = Application::cucumber()
        .steps(Collection::new().given(None, Regex::new("^work$").unwrap(), body))
        .before(move |_, _, _, world| {
            world.seen = seen.clone();
            world.config = config.clone();
            Box::pin(async {})
        })
        .max_concurrent_scenarios(2)
        .retries(if world.config == "retry" { 1 } else { 0 })
        .with_writer(writer::Basic::new(&mut output, Coloring::Never, 0).summarized())
        .with_run_hooks(hooks)
        .with_default_cli()
        .run(&path)
        .await;
    assert_eq!(result.parsing_errors(), 0);
    world.hook_errors = result.hook_errors();
    world.step_errors = result.failed_steps();
    world.failed = result.execution_has_failed();
    assert_eq!(
        world.failed,
        world.hook_errors > 0 || world.config == "body fails"
    );
    assert_eq!(world.step_errors, usize::from(world.config == "body fails"));
}
#[then(expr = "the run lifecycle trace is {string}")]
fn trace(world: &mut Acceptance, expected: String) {
    assert_eq!(world.seen.lock().unwrap().trace.join(","), expected);
}
#[then(expr = "global hook statistics report {int} failures")]
fn statistics(world: &mut Acceptance, count: usize) {
    assert_eq!(world.hook_errors, count);
}
#[then("global records identify each actual callback and its result")]
fn records(world: &mut Acceptance) {
    let records = world.store.records();
    assert_eq!(records.len(), 4);
    let mut invocation_ids = HashSet::new();
    let mut definition_ids = HashSet::new();
    let run_id = world.store.current_run().unwrap();
    let scopes = world.seen.lock().unwrap().scopes.clone();
    for (index, record) in records.iter().enumerate() {
        assert_eq!(record.run_id, run_id);
        assert!(invocation_ids.insert(record.invocation_id));
        assert!(definition_ids.insert(record.definition.id));
        assert_eq!(
            scopes[index],
            (record.run_id, record.invocation_id, record.definition.id)
        );
        assert!(record.finished_at >= record.started_at);
        assert_eq!(
            record.definition.kind,
            if index < 2 {
                RunHookKind::BeforeAll
            } else {
                RunHookKind::AfterAll
            }
        );
        let expected = match world.config.as_str() {
            "before fails" | "before construct fails" => record.definition.name == "before-one",
            "before both" => record.definition.kind == RunHookKind::BeforeAll,
            "after fails" | "after construct fails" => record.definition.name == "after-two",
            "all fail" => true,
            _ => false,
        };
        assert_eq!(record.result.is_err(), expected);
        if let Err(message) = &record.result {
            assert!(message.contains(&record.definition.name));
        }
    }
    assert_eq!(world.store.failures_for(run_id), world.hook_errors);
}
#[when("an isolated process runs it using run_and_exit")]
fn process(world: &mut Acceptance) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("run.feature");
    std::fs::write(&path, feature(&world.config)).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .env("CUCUMBER_RUN_HOOK_CHILD", &world.config)
        .env("CUCUMBER_RUN_HOOK_PATH", &path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    world.child_error = String::from_utf8_lossy(&output.stderr).into_owned();
}
#[then("the process fails with a global hook diagnostic")]
fn diagnostic(world: &mut Acceptance) {
    assert!(
        world.child_error.contains("1 hook error"),
        "{}",
        world.child_error
    );
}
#[given("a reusable run-hook registry and observation store")]
fn reusable(world: &mut Acceptance) {
    world.config = "reuse".into();
}
#[when("one failing run and one passing run execute sequentially")]
async fn reuse(world: &mut Acceptance) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("run.feature");
    std::fs::write(&path, "Feature: Reuse\n").unwrap();
    let failing = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let flag = failing.clone();
    let hooks = RunHooks::new()
        .with_store(world.store.clone())
        .after("cleanup", move |_| {
            let flag = flag.clone();
            Box::pin(async move {
                assert!(
                    !flag.load(std::sync::atomic::Ordering::SeqCst),
                    "first run cleanup failed"
                );
            })
        });
    let mut first_output = Vec::new();
    let first = Application::cucumber()
        .with_writer(writer::Basic::new(&mut first_output, Coloring::Never, 0).summarized())
        .with_run_hooks(hooks.clone())
        .with_default_cli()
        .run(&path)
        .await;
    assert_eq!(first.hook_errors(), 1);
    failing.store(false, std::sync::atomic::Ordering::SeqCst);
    let mut second_output = Vec::new();
    let second = Application::cucumber()
        .with_writer(writer::Basic::new(&mut second_output, Coloring::Never, 0).summarized())
        .with_run_hooks(hooks)
        .with_default_cli()
        .run(&path)
        .await;
    assert_eq!(first.hook_errors(), 1);
    assert!(first.execution_has_failed());
    assert_eq!(second.hook_errors(), 0);
    assert!(!second.execution_has_failed());
    let records = world.store.records();
    assert_eq!(records.len(), 2);
    assert_ne!(records[0].run_id, records[1].run_id);
    assert_eq!(records[0].definition.id, records[1].definition.id);
    assert!(records[0].result.is_err());
    assert!(records[1].result.is_ok());
    world.separate = true;
}
#[then("their completed statistics and callback records remain separate")]
fn separated(world: &mut Acceptance) {
    assert!(world.separate);
}
async fn child(config: String) {
    let world = Acceptance {
        config: config.clone(),
        ..Default::default()
    };
    let hooks = hooks(&world);
    let seen = world.seen.clone();
    Application::cucumber()
        .steps(Collection::new().given(None, Regex::new("^work$").unwrap(), body))
        .before(move |_, _, _, world| {
            world.seen = seen.clone();
            world.config = config.clone();
            Box::pin(async {})
        })
        .with_run_hooks(hooks)
        .with_default_cli()
        .run_and_exit(std::env::var("CUCUMBER_RUN_HOOK_PATH").unwrap())
        .await;
}
#[tokio::main]
async fn main() {
    if let Ok(config) = std::env::var("CUCUMBER_RUN_HOOK_CHILD") {
        child(config).await;
    } else {
        Acceptance::cucumber()
            .fail_on_skipped()
            .run_and_exit("tests/features/run_hooks")
            .await;
    }
}
