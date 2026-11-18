//! Phase27 acceptance: actual registered support code and official Query resolution.
use std::{io::Write as _, process::{Command, Stdio}, sync::{LazyLock, Mutex}};
use cucumber::{given, when, then, World as _, step::{Collection, Context, Matching}};
use futures::{Stream, future::LocalBoxFuture};
use regex::Regex;
use serde_json::{Value, json};

#[derive(Debug, Default, cucumber::World)]
struct Report { mode: String, output: Vec<u8>, consumed: Value }
#[derive(Debug, Default, cucumber::World)]
struct Execution;
static CALLS: LazyLock<Mutex<usize>> = LazyLock::new(Default::default);
#[given(expr = "paid {int}")]
fn paid(_: &mut Execution, amount: i64) { assert_eq!(amount, 42); }
fn action(_: &mut Execution, context: Context) -> LocalBoxFuture<'_, ()> {
    let count = { let mut calls = CALLS.lock().unwrap(); *calls += 1; *calls };
    Box::pin(async move {
        assert!(context.step.value != "failure", "deliberate first-step failure");
        assert!(context.step.value != "retry" || count > 1, "deliberate first-attempt failure");
    })
}
fn obsolete(_: &mut Execution, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async { panic!("replaced definition must never execute"); })
}
// Intentionally implements only the original Runner contract: the additive
// metadata sink must remain optional for third-party runners.
struct Custom(cucumber::runner::Basic<Execution>);
impl cucumber::Runner<Execution> for Custom {
    type Cli = cucumber::runner::basic::Cli;
    type EventStream = <cucumber::runner::Basic<Execution> as cucumber::Runner<Execution>>::EventStream;
    fn run<S>(self, features: S, cli: Self::Cli) -> Self::EventStream
    where S: Stream<Item = cucumber::parser::Result<cucumber::gherkin::Feature>> + 'static {
        self.0.run(features, cli)
    }
}
fn registration(pattern: &str) -> Collection<Execution> {
    Collection::new().given(None, Regex::new(pattern).unwrap(), action)
}
#[given(expr = "a support-code run configured as {string}")]
fn configure(report: &mut Report, mode: String) { report.mode = mode; }
#[when("that run completes and the pinned official Query reads its Messages")]
async fn execute(report: &mut Report) {
    *CALLS.lock().unwrap() = 0;
    let mut source_step = "Given ready";
    let mut steps = registration("^ready$");
    match report.mode.as_str() {
        "macro" => { steps = Execution::collection(); source_step = "Given paid 42"; }
        "ambiguous" => { steps = steps.given(None, Regex::new("ready").unwrap(), action); }
        "undefined" => { source_step = "Given missing"; }
        "keyword-specific" | "keyword-independent" | "independent-late" => { source_step = "When ready"; }
        "cross-keyword" => { steps = steps.when(None, Regex::new("^ready$").unwrap(), action); }
        "duplicate" => {
            steps = Collection::new().given(None, Regex::new("^ready$").unwrap(), obsolete)
                .given(None, Regex::new("^ready$").unwrap(), action);
        }
        "retry" => { steps = registration("^retry$"); source_step = "Given retry"; }
        "replacement" => { steps = registration("^obsolete$").matching(Matching::KeywordIndependent); source_step = "When ready\n    Given ready"; }
        "skipped" => { steps = registration("^(failure|ready)$"); source_step = "Given failure\n    And ready"; }
        "unique" | "custom" | "builder-chain" => {}
        other => panic!("unknown fixture {other}"),
    }
    if matches!(report.mode.as_str(), "keyword-independent" | "cross-keyword") {
        steps = steps.matching(Matching::KeywordIndependent);
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("support-code.feature");
    std::fs::write(&path, format!("Feature: Definition discovery\n  Scenario: reported\n    {source_step}\n")).unwrap();
    if report.mode == "custom" {
        Execution::cucumber().with_runner(Custom(cucumber::runner::Basic::default().steps(steps)))
            .with_messages(&mut report.output).with_default_cli().run(&path).await;
    } else {
        let mut runner = Execution::cucumber().steps(steps).retries(if report.mode == "retry" { 1 } else { 0 })
            .with_messages(&mut report.output);
        if report.mode == "replacement" {
            // The collection and its default keyword-specific policy replace
            // both earlier registrations and earlier matching policy.
            runner = runner.steps(Collection::new().when(None, Regex::new("^ready$").unwrap(), action));
        }
        if report.mode == "independent-late" {
            runner = runner.step_matching(Matching::KeywordIndependent);
        }
        if report.mode == "builder-chain" {
            runner.before(|_, _, _, _| Box::pin(async {}))
                .after(|_, _, _, _, _| Box::pin(async {}))
                .which_scenario(|_, _, _| cucumber::runner::ScenarioType::Serial)
                .with_hooks(cucumber::runner::Hooks::new())
                .with_step_hooks(cucumber::runner::StepHooks::new())
                .with_run_hooks(cucumber::runner::RunHooks::new())
                .with_default_cli().run(&path).await;
        } else {
            runner.with_default_cli().run(&path).await;
        }
    }
    let mut process = Command::new("node").arg("tests/interop/support-code.mjs")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    process.stdin.take().unwrap().write_all(&report.output).unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success(), "official Query failed: {}", String::from_utf8_lossy(&output.stderr));
    report.consumed = serde_json::from_slice(&output.stdout).unwrap();
}
fn first_step(report: &Report) -> &Value { &report.consumed["attempts"][0]["steps"][0] }
#[then("the consumer resolves the actual compiled regex and Rust registration location")]
fn macro_source(report: &mut Report) {
    let snapshot = Execution::collection().support_code();
    let registration = &snapshot.definitions()[0];
    let location = registration.location().unwrap();
    assert!(location.path.ends_with("tests/messages_support_code.rs"));
    assert!(location.line > 0 && location.column > 0);
    let step = first_step(report);
    let definition = &step["unambiguous"];
    assert_eq!(step["status"], "PASSED");
    assert_eq!(definition["pattern"], json!({"source":registration.regex().as_str(), "type":"REGULAR_EXPRESSION"}));
    assert_eq!(definition["sourceReference"], json!({"uri":location.path, "location":{"line":location.line,"column":location.column}}));
    assert!(step.get("stepMatchArgumentsLists").is_none());
}
#[then(expr = "the consumer resolves {int} definitions and observes {string}")]
fn matched(report: &mut Report, count: usize, status: String) {
    let step = first_step(report);
    assert_eq!(step["definitions"].as_array().unwrap().len(), count);
    assert_eq!(step["ids"].as_array().unwrap().len(), count);
    assert_eq!(step["status"], status);
    assert_eq!(step.get("unambiguous").is_some(), count == 1);
}
#[then("only the final replacement collection is reported and executed")]
fn replacement(report: &mut Report) {
    let definitions = report.consumed["definitions"].as_array().unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0]["pattern"]["source"], "^ready$");
    assert_eq!(first_step(report)["status"], "PASSED");
    assert_eq!(*CALLS.lock().unwrap(), 1);
    let skipped_by_policy = &report.consumed["attempts"][0]["steps"][1];
    assert_eq!(skipped_by_policy["status"], "UNDEFINED");
    assert_eq!(skipped_by_policy["ids"], json!([]));
}
#[then("the consumer resolves one raw definition with an empty source reference")]
fn raw(report: &mut Report) {
    assert_eq!(report.consumed["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(first_step(report)["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(first_step(report)["unambiguous"]["sourceReference"], json!({}));
    assert_eq!(first_step(report)["status"], "PASSED");
    assert_eq!(*CALLS.lock().unwrap(), 1);
}
#[then("both attempts reference the same single registered definition")]
fn retry(report: &mut Report) {
    let attempts = report.consumed["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(report.consumed["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(attempts[0]["steps"][0]["ids"], attempts[1]["steps"][0]["ids"]);
    assert_eq!(attempts[0]["steps"][0]["status"], "FAILED");
    assert_eq!(attempts[1]["steps"][0]["status"], "PASSED");
}
#[then("unsupported definition metadata is omitted rather than fabricated")]
fn omitted(report: &mut Report) {
    assert!(report.consumed["definitions"].as_array().unwrap().is_empty());
    assert!(first_step(report).get("ids").is_none());
    assert_eq!(first_step(report)["status"], "PASSED");
}
#[then("the skipped planned step still resolves its actual definition")]
fn skipped(report: &mut Report) {
    let steps = report.consumed["attempts"][0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 2); assert_eq!(steps[0]["status"], "FAILED");
    assert_eq!(steps[1]["status"], "SKIPPED");
    assert_eq!(steps[1]["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(steps[0]["ids"], steps[1]["ids"]);
}
#[then("every emitted envelope satisfies the pinned Messages schema")]
fn schema(report: &mut Report) {
    let schema = serde_json::from_str(include_str!("fixtures/messages/messages-34.2.1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for line in String::from_utf8(report.output.clone()).unwrap().lines() {
        let message: Value = serde_json::from_str(line).unwrap();
        assert!(validator.is_valid(&message), "invalid envelope {message}");
        if let Some(case) = message.get("testCase") {
            for step in case["testSteps"].as_array().unwrap() { assert!(step.get("stepMatchArgumentsLists").is_none()); }
        }
    }
}
#[tokio::main]
async fn main() {
    Report::cucumber().max_concurrent_scenarios(1).fail_on_skipped()
        .run_and_exit("tests/features/messages_support_code").await;
}
