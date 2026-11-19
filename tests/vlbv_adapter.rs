//! Phase39: real Cucumber child processes and explicit VLBV receipt projection.
use std::{
    io::Write as _,
    path::PathBuf,
    process::Command,
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use cucumber::{World as _, WriterExt as _, given, then, when};
use serde_json::{Value, json};
static EVIDENCE: OnceLock<PathBuf> = OnceLock::new();
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    fixture: String,
    result: Value,
}
#[derive(Debug, Default, cucumber::World)]
struct Application;
static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
#[given(regex = "^observe (.*)$")]
fn observe(_: &mut Application, mode: String) {
    let actual =
        if mode == "retry" && ATTEMPTS.fetch_add(1, Ordering::SeqCst) > 0 {
            "expected"
        } else if matches!(
            mode.as_str(),
            "pass" | "exit1" | "exit2" | "dry" | "global"
        ) {
            "expected"
        } else {
            "wrong"
        };
    assert_eq!(actual, "expected", "independent fixed truth comparison");
}
#[then("check the observation")]
fn check(_: &mut Application) {}
#[given(
    "a real Cucumber run with frozen v2 contract and complete assertion \
        control and cleanup receipts"
)]
fn complete(w: &mut Acceptance) {
    w.fixture = "complete".into();
}
#[given(expr = "boundary evidence for {string}")]
fn fixture(w: &mut Acceptance, value: String) {
    w.fixture = value;
}
#[when("the read-only Messages converter projects the boundary")]
fn project(w: &mut Acceptance) {
    let evidence = EVIDENCE
        .get_or_init(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("docs/compatibility/evidence/vlbv-adapter")
                .join(format!(
                    "boundary-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                ))
        })
        .join(w.fixture.replace(' ', "_"));
    let output = Command::new("node")
        .arg("tests/interop/vlbv/boundary-fixture.mjs")
        .arg(std::env::current_exe().unwrap())
        .arg(&w.fixture)
        .arg(&evidence)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    w.result = serde_json::from_slice(&output.stdout)
        .expect("Phase39 fixture producer output must be complete JSON");
    let schema: Value = serde_json::from_str(include_str!(
        "fixtures/vlbv/adopted-contract.v2.schema.json"
    ))
    .expect("copied canonical adopted-contract v2 schema must parse");
    let validator = jsonschema::validator_for(&schema)
        .expect("Phase39 referenced schema must compile");
    let errors = validator
        .iter_errors(&w.result["contract"])
        .map(|e| e.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "contract fixture: {errors:?}");
    let schema: Value = serde_json::from_str(include_str!(
        "../docs/compatibility/phase39-receipts.schema.json"
    ))
    .expect("Phase39 adapter receipt schema must parse");
    let validator = jsonschema::validator_for(&schema)
        .expect("Phase39 referenced schema must compile");
    let errors = validator
        .iter_errors(&w.result["bundle"])
        .map(|e| e.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "receipt fixture: {errors:?}");
}
fn result(w: &Acceptance) -> &Value {
    &w.result["output"]
}
#[then(expr = "the adopted scenario verdict is {string}")]
fn verdict(w: &mut Acceptance, expected: String) {
    assert_eq!(result(w)["scenarios"][0]["status"], expected, "{}", w.result);
    let exit = if expected == "PASS" {
        0
    } else if expected == "FAIL" {
        3
    } else {
        2
    };
    assert_eq!(w.result["exit"], exit);
}
#[then(
    "the canonical result fields and all original evidence references are \
        retained"
)]
fn canonical(w: &mut Acceptance) {
    for key in [
        "phase",
        "projectId",
        "runId",
        "contractDigest",
        "sourceDigests",
        "executedBy",
        "budgetConsumption",
    ] {
        assert!(!result(w)[key].is_null(), "{key}");
    }
    assert_eq!(w.result["canonicalReaderExit"], 0, "{}", w.result);
    assert_eq!(result(w)["adapter"]["inputs"], w.result["bundle"]);
}
#[then("the actual failed observation remains in the attempt ledger")]
fn failure(w: &mut Acceptance) {
    assert!(
        result(w)["scenarios"][0]["evidence"]["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["status"] == "FAIL"
                && !a["assertions"][0]["observed"]
                    .as_str()
                    .unwrap()
                    .is_empty())
    );
}
#[then(
    "the exercised scenario passes and the absent adopted scenario is \
        NOT_EXERCISED"
)]
fn denominator(w: &mut Acceptance) {
    assert_eq!(result(w)["scenarios"].as_array().unwrap().len(), 2);
    assert_eq!(result(w)["scenarios"][0]["status"], "PASS");
    assert_eq!(result(w)["scenarios"][1]["status"], "NOT_EXERCISED");
}
#[then("every failed and successful attempt is retained")]
fn attempts(w: &mut Acceptance) {
    let attempts = result(w)["adapter"]["runs"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["attempts"].as_array().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(attempts.len(), 2, "{}", w.result);
    assert!(
        attempts
            .iter()
            .any(|a| a["steps"][0]["testStepResult"]["status"] == "FAILED")
    );
    assert!(
        attempts
            .iter()
            .any(|a| a["steps"][0]["testStepResult"]["status"] == "PASSED")
    );
}
#[then(expr = "the refusal identifies {string}")]
fn reason(w: &mut Acceptance, expected: String) {
    assert!(
        result(w)["scenarios"][0]["observed"]["reasons"]
            .to_string()
            .contains(&expected),
        "{}",
        w.result
    );
}
#[then(
    "a legacy receipt without the stream binding is also blocked with \
        migration guidance"
)]
fn legacy_binding(w: &mut Acceptance) {
    assert_eq!(w.result["migration"]["exit"], 2);
    let scenario = &w.result["migration"]["output"]["scenarios"][0];
    assert_eq!(scenario["status"], "BLOCKED");
    assert!(
        scenario["observed"]["reasons"]
            .to_string()
            .contains("record process.messages {path, sha256}"),
        "{}",
        w.result
    );
}
#[then(
    "all interleaved adopted scenarios pass with separate attempt identities"
)]
fn interleaved(w: &mut Acceptance) {
    let scenarios = result(w)["scenarios"].as_array().unwrap();
    assert_eq!(scenarios.len(), 2);
    assert!(scenarios.iter().all(|s| s["status"] == "PASS"), "{}", w.result);
    assert_ne!(
        scenarios[0]["evidence"]["attempts"][0]["testCaseStartedId"],
        scenarios[1]["evidence"]["attempts"][0]["testCaseStartedId"]
    );
}
#[then("every canonical input and sentinel remains unchanged")]
fn unchanged(w: &mut Acceptance) {
    assert_eq!(w.result["unchanged"], true);
}
#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--vlbv-child") {
        let mode = &args[2];
        let input = &args[3];
        let mut output = Vec::new();
        let store = cucumber::runner::RunHookStore::new();
        let fail_global = mode == "global";
        let hooks = cucumber::runner::RunHooks::new()
            .with_store(store.clone())
            .after("actual global cleanup", move |_| {
                Box::pin(async move {
                    assert!(!fail_global, "global cleanup failed");
                })
            });
        let sources = cucumber::parser::SourceStore::new();
        let writer = cucumber::writer::Messages::raw_with_source_store(
            &mut output,
            sources.clone(),
        )
        .with_run_hook_store(store)
        .normalized();
        Application::cucumber()
            .retries(if mode == "retry" { 1 } else { 0 })
            .execution_mode(if mode == "dry" {
                cucumber::execution::ExecutionMode::DryRun
            } else {
                cucumber::execution::ExecutionMode::Run
            })
            .with_source_store(sources)
            .with_writer(writer)
            .with_run_hooks(hooks)
            .with_default_cli()
            .run(input)
            .await;
        let messages = String::from_utf8(output.clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        std::io::stdout().write_all(&output).unwrap();
        let success = messages
            .iter()
            .find_map(|m| m.get("testRunFinished"))
            .unwrap()["success"]
            == json!(true);
        std::process::exit(if mode == "exit2" {
            2
        } else if mode == "exit1" {
            1
        } else if success {
            0
        } else {
            1
        });
    }
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/vlbv_adapter")
        .await;
}
