//! Phase26 actual global hook records exchanged with the official consumer.
#[path = "support/oracle.rs"]
mod oracle;

use std::{collections::HashSet, io::Write as _, process::Stdio};

use cucumber::{
    World as _, WriterExt as _, given,
    runner::{RunHookKind, RunHookRecord, RunHookStore, RunHooks},
    step::{Collection, Context},
    then, when,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    reports: Vec<Vec<Value>>,
    records: Vec<Vec<RunHookRecord>>,
}
#[derive(Debug, Default, cucumber::World)]
struct Application;
fn body(_: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}
async fn execute(
    mode: &str,
    store: RunHookStore,
) -> (Vec<Value>, Vec<RunHookRecord>) {
    let before = matches!(mode, "setup failure" | "both failures");
    let after = matches!(mode, "cleanup failure" | "both failures");
    let hooks = RunHooks::new()
        .with_store(store.clone())
        .before("setup-one", move |_| {
            Box::pin(async move { assert!(!before, "global setup failed") })
        })
        .before("setup-two", |_| Box::pin(async {}))
        .after("cleanup-one", |_| Box::pin(async {}))
        .after("cleanup-two", move |_| {
            Box::pin(async move { assert!(!after, "global cleanup failed") })
        });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("global.feature");
    std::fs::write(
        &path,
        if mode == "empty input" {
            "# no feature\n"
        } else {
            "Feature: Global\n Scenario: Work\n  Given action\n"
        },
    )
    .unwrap();
    let sources = cucumber::parser::SourceStore::new();
    let mut output = Vec::new();
    let writer = cucumber::writer::Messages::raw_with_source_store(
        &mut output,
        sources.clone(),
    )
    .with_run_hook_store(store.clone())
    .normalized();
    Application::cucumber()
        .steps(Collection::new().given(
            None,
            Regex::new("^action$").unwrap(),
            body,
        ))
        .with_gherkin43()
        .unwrap()
        .with_source_store(sources)
        .with_writer(writer)
        .with_run_hooks(hooks)
        .with_default_cli()
        .run(&path)
        .await;
    let records = store.records_for(store.current_run().unwrap());
    (
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect(),
        records,
    )
}
fn all<'a>(messages: &'a [Value], kind: &str) -> Vec<&'a Value> {
    messages.iter().filter_map(|m| m.get(kind)).collect()
}
fn validate(messages: &[Value]) {
    let schema: Value = serde_json::from_str(include_str!(
        "fixtures/messages/messages-34.2.1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for message in messages {
        let errors: Vec<_> =
            validator.iter_errors(message).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
}
#[given(expr = "a run with global hooks configured for {string}")]
fn select(world: &mut Acceptance, mode: String) {
    world.mode = mode;
}
#[when("its shared global-hook records are reported as Messages")]
async fn run(world: &mut Acceptance) {
    let (m, r) = execute(&world.mode, RunHookStore::new()).await;
    world.reports.push(m);
    world.records.push(r);
}
#[then(
    "every observed global invocation has a definition and paired lifecycle"
)]
fn lifecycle(world: &mut Acceptance) {
    let m = &world.reports[0];
    validate(m);
    let records = &world.records[0];
    assert_eq!(records.len(), 4);
    let starts = all(m, "testRunHookStarted");
    let ends = all(m, "testRunHookFinished");
    let definitions = all(m, "hook");
    assert_eq!(starts.len(), 4);
    assert_eq!(ends.len(), 4);
    assert_eq!(definitions.len(), 4);
    for record in records {
        let id = format!("native-run-hook-invocation-{}", record.invocation_id);
        let start = starts.iter().find(|s| s["id"] == id).unwrap();
        let end =
            ends.iter().find(|e| e["testRunHookStartedId"] == id).unwrap();
        let definition =
            definitions.iter().find(|d| d["id"] == start["hookId"]).unwrap();
        assert_eq!(definition["name"], record.definition.name);
        assert_eq!(
            definition["type"],
            if record.definition.kind == RunHookKind::BeforeAll {
                "BEFORE_TEST_RUN"
            } else {
                "AFTER_TEST_RUN"
            }
        );
        assert_eq!(
            end["result"]["status"],
            if record.result.is_ok() { "PASSED" } else { "FAILED" }
        );
        if let Err(error) = &record.result {
            assert_eq!(end["result"]["message"], *error);
        }
        for (value, at) in [
            (&start["timestamp"], record.started_at),
            (&end["timestamp"], record.finished_at),
        ] {
            let actual = at.duration_since(std::time::UNIX_EPOCH).unwrap();
            assert_eq!(value["seconds"].as_u64(), Some(actual.as_secs()));
            assert_eq!(
                value["nanos"].as_u64(),
                Some(actual.subsec_nanos().into())
            );
        }
        let elapsed =
            record.finished_at.duration_since(record.started_at).unwrap();
        assert_eq!(
            end["result"]["duration"]["nanos"].as_u64(),
            Some(elapsed.subsec_nanos().into())
        );
    }
    assert_eq!(
        all(m, "testStepFinished")
            .iter()
            .filter(|e| e["testStepResult"]["status"] == "FAILED")
            .count(),
        0
    );
}
#[then("global setup is reported before scenarios and cleanup after them")]
fn ordering(world: &mut Acceptance) {
    let m = &world.reports[0];
    let names = all(m, "hook")
        .iter()
        .map(|h| h["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["setup-one", "setup-two", "cleanup-two", "cleanup-one"]
    );
    let starts = m
        .iter()
        .enumerate()
        .filter(|(_, m)| m.get("testRunHookStarted").is_some())
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let ends = m
        .iter()
        .enumerate()
        .filter(|(_, m)| m.get("testRunHookFinished").is_some())
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    for (i, message) in m.iter().enumerate() {
        if message.get("testCaseStarted").is_some() {
            assert!(ends[1] < i);
        }
        if message.get("testCaseFinished").is_some() {
            assert!(i < starts[2]);
        }
    }
    assert!(m.last().unwrap().get("testRunFinished").is_some());
}
#[then(
    expr = "the report contains {int} scenario attempts and success {string}"
)]
fn outcomes(world: &mut Acceptance, cases: usize, success: String) {
    let m = &world.reports[0];
    assert_eq!(all(m, "testCaseStarted").len(), cases);
    assert_eq!(all(m, "testRunFinished")[0]["success"], success == "true");
}
#[then("the official consumer resolves the global-hook outcomes")]
fn consume(world: &mut Acceptance) {
    let mut child = oracle::command("tests/interop/run-hooks.mjs")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut input = child.stdin.take().unwrap();
        for message in &world.reports[0] {
            writeln!(input, "{message}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let results: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(results.len(), 4);
    for (observed, record) in results.iter().zip(&world.records[0]) {
        assert_eq!(observed["hook"]["name"], record.definition.name);
        assert_eq!(
            observed["result"]["status"],
            if record.result.is_ok() { "PASSED" } else { "FAILED" }
        );
        assert_eq!(observed["started"]["testRunStartedId"], "run-0");
    }
}
#[given("two sequential runs sharing a global-hook store")]
fn reuse(world: &mut Acceptance) {
    world.mode = "reuse".into();
}
#[when("each run is reported with its own Messages writer")]
async fn twice(world: &mut Acceptance) {
    let store = RunHookStore::new();
    for mode in ["both failures", "passing"] {
        let (m, r) = execute(mode, store.clone()).await;
        world.reports.push(m);
        world.records.push(r);
    }
}
#[then("each report contains only its own observed global-hook invocations")]
fn isolated(world: &mut Acceptance) {
    let mut seen = HashSet::new();
    for (m, records) in world.reports.iter().zip(&world.records) {
        validate(m);
        let starts = all(m, "testRunHookStarted");
        assert_eq!(starts.len(), records.len());
        for start in starts {
            assert!(seen.insert(start["id"].as_str().unwrap().to_owned()));
            assert!(records.iter().any(|r| start["id"]
                == format!("native-run-hook-invocation-{}", r.invocation_id)));
        }
    }
}
#[then("both reports have complete unsuccessful or successful final outcomes")]
fn reuse_outcomes(world: &mut Acceptance) {
    assert_eq!(all(&world.reports[0], "testRunFinished")[0]["success"], false);
    assert_eq!(all(&world.reports[1], "testRunFinished")[0]["success"], true);
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_run_hooks")
        .await;
}
