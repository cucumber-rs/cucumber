//! Phase44 actual Messages rerun collection and native second execution.
use cucumber::{
    World as _, given,
    step::{Collection, Context},
    then, when,
    writer::messages::{
        rerun::{Collector, Error, Plan, Status},
        source::SourceGraph,
    },
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
static COUNTS: LazyLock<Mutex<HashMap<String, usize>>> =
    LazyLock::new(Default::default);
static EXECUTED: Mutex<Vec<String>> = Mutex::new(Vec::new());
static RECOVER: AtomicBool = AtomicBool::new(false);
#[derive(Debug, Default, cucumber::World)]
struct Application;
fn body(_: &mut Application, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        EXECUTED.lock().unwrap().push(context.step.value.clone());
        assert!(
            context.step.value != "fail" || RECOVER.load(Ordering::SeqCst),
            "actual body failure"
        );
    })
}
fn steps() -> Collection<Application> {
    Collection::new().given(
        None,
        Regex::new("^(work|fail|pass|excluded)$").unwrap(),
        body,
    )
}
fn after<'a>(
    _: &'a gherkin::Feature,
    _: Option<&'a gherkin::Rule>,
    scenario: &'a gherkin::Scenario,
    _: &'a cucumber::event::ScenarioFinished,
    _: Option<&'a mut Application>,
) -> LocalBoxFuture<'a, ()> {
    Box::pin(async move {
        let attempt = {
            let mut counts = COUNTS.lock().unwrap();
            let count = counts.entry(scenario.name.clone()).or_default();
            *count += 1;
            *count
        };
        assert!(
            scenario.name == "recovered" && attempt > 1,
            "actual after-hook failure"
        );
    })
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    status: String,
    messages: Vec<Value>,
    plan: Option<Plan>,
    error: Option<Error>,
    directory: Option<tempfile::TempDir>,
    path: PathBuf,
    second: Vec<Value>,
    original: Vec<Value>,
    launched: bool,
}
fn parse(bytes: Vec<u8>) -> Vec<Value> {
    String::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn begin() -> Vec<Value> {
    vec![
        json!({"testRunStarted":{"id":"run","timestamp":{"seconds":0,"nanos":0}}}),
    ]
}
fn finish(messages: &mut Vec<Value>, success: bool) {
    messages.push(json!({"testRunFinished":{"testRunStartedId":"run","success":success,"timestamp":{"seconds":1,"nanos":0}}}));
}
fn add_case(
    messages: &mut Vec<Value>,
    uri: &str,
    label: &str,
    status: &str,
    completed: bool,
) {
    let source = if status == "UNKNOWN" && completed {
        "Feature: Protocol\n Scenario: case\n"
    } else {
        "Feature: Protocol\n Scenario: case\n  Given work\n"
    };
    let graph = SourceGraph::parse_gherkin43(uri, source, "en").unwrap();
    messages.push(json!({"gherkinDocument":graph.gherkin_document()}));
    let mut pickle = graph.pickles()[0].clone();
    pickle["id"] = json!(format!("pickle-{label}"));
    let step_ids: Vec<_> = pickle["steps"].as_array().unwrap().iter().enumerate().map(|(index,step)|json!({"id":format!("step-{label}-{index}"),"pickleStepId":step["id"]})).collect();
    messages.push(json!({"pickle":pickle}));
    messages.push(json!({"testCase":{"id":format!("case-{label}"),"pickleId":format!("pickle-{label}"),"testSteps":step_ids}}));
    messages.push(json!({"testCaseStarted":{"id":format!("start-{label}"),"testCaseId":format!("case-{label}"),"attempt":0,"timestamp":{"seconds":0,"nanos":0}}}));
    if completed {
        for step in step_ids {
            messages.push(json!({"testStepFinished":{"testCaseStartedId":format!("start-{label}"),"testStepId":step["id"],"timestamp":{"seconds":0,"nanos":0},"testStepResult":{"status":status,"duration":{"seconds":0,"nanos":0}}}}));
        }
        messages.push(json!({"testCaseFinished":{"testCaseStartedId":format!("start-{label}"),"willBeRetried":false,"timestamp":{"seconds":0,"nanos":0}}}));
    }
}
#[given(
    expr = "a completed protocol case with status {string} and run success {string}"
)]
fn protocol(w: &mut Acceptance, status: String, success: String) {
    w.status = status.clone();
    w.messages = begin();
    add_case(&mut w.messages, "case.feature", "one", &status, true);
    finish(&mut w.messages, success == "true");
    // Standard Messages permits an absent TestRunStarted ID; preserve absence.
    if status == "SKIPPED" {
        w.messages[0]["testRunStarted"].as_object_mut().unwrap().remove("id");
        w.messages.last_mut().unwrap()["testRunFinished"]
            .as_object_mut()
            .unwrap()
            .remove("testRunStartedId");
    }
}
fn file(w: &mut Acceptance, source: &str) {
    let directory = tempfile::tempdir().unwrap();
    w.path = directory.path().join("rerun.feature");
    std::fs::write(&w.path, source).unwrap();
    w.directory = Some(directory);
}
#[given("a real application with recovering and exhausted cases")]
async fn retries(w: &mut Acceptance) {
    COUNTS.lock().unwrap().clear();
    EXECUTED.lock().unwrap().clear();
    file(
        w,
        "Feature: Actual retry hooks\n Scenario: recovered\n  Given work\n Scenario: exhausted\n  Given work\n",
    );
    let mut output = Vec::new();
    Application::cucumber()
        .steps(steps())
        .after(after)
        .retries(1)
        .max_concurrent_scenarios(2)
        .with_messages(&mut output)
        .with_default_cli()
        .run(&w.path)
        .await;
    w.messages = parse(output);
}
#[given("a real filtered concurrent Rule outline with duplicate row values")]
async fn outline(w: &mut Acceptance) {
    RECOVER.store(false, Ordering::SeqCst);
    EXECUTED.lock().unwrap().clear();
    file(
        w,
        "Feature: Actual rows\n Rule: resources\n  Scenario Outline: result <outcome>\n   Given <outcome>\n   Examples: selected\n    | outcome |\n    | fail |\n    | pass |\n    | fail |\n    | excluded |\n",
    );
    let mut output = Vec::new();
    Application::cucumber()
        .steps(steps())
        .with_gherkin43()
        .unwrap()
        .max_concurrent_scenarios(3)
        .with_messages(&mut output)
        .with_default_cli()
        .filter_run(&w.path, |_, _, scenario| {
            !scenario.name.contains("excluded")
        })
        .await;
    w.messages = parse(output);
    w.original = w.messages.clone();
}
#[given("completed cases sharing source locations across multiple URIs")]
fn groups(w: &mut Acceptance) {
    w.messages = begin();
    for (index, uri) in
        ["z.feature", "a.feature", "z.feature", "a.feature", "2", "1"]
            .into_iter()
            .enumerate()
    {
        add_case(
            &mut w.messages,
            uri,
            &format!("group-{index}"),
            "FAILED",
            true,
        );
    }
    finish(&mut w.messages, false);
}
#[given("a run that finishes after a case starts without its finish event")]
fn interrupted(w: &mut Acceptance) {
    w.messages = begin();
    add_case(&mut w.messages, "case.feature", "one", "UNKNOWN", false);
    finish(&mut w.messages, false);
}
#[given("a protocol stream without testRunFinished")]
fn incomplete(w: &mut Acceptance) {
    w.messages = begin();
    add_case(&mut w.messages, "case.feature", "one", "PASSED", true);
}
#[given(expr = "a protocol stream with a missing {string} reference")]
fn missing(w: &mut Acceptance, reference: String) {
    protocol(w, "FAILED".into(), "false".into());
    match reference.as_str() {
        "testCase" => {
            w.messages
                .iter_mut()
                .find_map(|e| e.get_mut("testCaseStarted"))
                .unwrap()["testCaseId"] = json!("missing-case")
        }
        "pickle" => {
            w.messages.iter_mut().find_map(|e| e.get_mut("testCase")).unwrap()
                ["pickleId"] = json!("missing-pickle")
        }
        "document" => w.messages.retain(|e| e.get("gherkinDocument").is_none()),
        "astNode" => {
            w.messages.iter_mut().find_map(|e| e.get_mut("pickle")).unwrap()["astNodeIds"] =
                json!(["missing-node"])
        }
        _ => panic!("unexpected missing reference"),
    }
}
#[given(
    "a failed protocol run with parse and global-hook failures but no case attempts"
)]
fn run_failure(w: &mut Acceptance) {
    w.messages = begin();
    w.messages.push(json!({"parseError":{"source":{"uri":"broken.feature","location":{"line":1,"column":1}},"message":"invalid source"}}));
    w.messages.push(json!({"hook":{"id":"global-hook","type":"BEFORE_TEST_RUN","sourceReference":{}}}));
    w.messages.push(json!({"testRunHookStarted":{"id":"global","testRunStartedId":"run","hookId":"global-hook","timestamp":{"seconds":0,"nanos":0}}}));
    w.messages.push(json!({"testRunHookFinished":{"testRunHookStartedId":"global","result":{"status":"FAILED","duration":{"seconds":0,"nanos":0}},"timestamp":{"seconds":0,"nanos":0}}}));
    finish(&mut w.messages, false);
}
#[given("a real application whose selected cases all pass")]
async fn passing(w: &mut Acceptance) {
    RECOVER.store(true, Ordering::SeqCst);
    file(
        w,
        "Feature: Passing\n Scenario: first\n  Given pass\n Scenario: second\n  Given pass\n",
    );
    let mut output = Vec::new();
    Application::cucumber()
        .steps(steps())
        .with_messages(&mut output)
        .with_default_cli()
        .run(&w.path)
        .await;
    w.messages = parse(output);
}
#[when("the application collects its rerun plan")]
fn collect(w: &mut Acceptance) {
    let mut collector = Collector::new();
    for envelope in &w.messages {
        if let Err(error) = collector.ingest(envelope) {
            w.error = Some(error);
            return;
        }
    }
    match collector.plan() {
        Ok(plan) => w.plan = Some(plan),
        Err(error) => w.error = Some(error),
    }
}
fn plan(w: &Acceptance) -> &Plan {
    assert!(w.error.is_none(), "{:?}", w.error);
    w.plan.as_ref().unwrap()
}
fn oracle(w: &Acceptance, separator: &str) -> Value {
    let mut child = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/interop/rerun/oracle.mjs"
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("official rerun oracle requires Node");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({"envelopes":w.messages,"separator":separator})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[then(expr = "the rerun selection contains {string}")]
fn membership(w: &mut Acceptance, expected: String) {
    let p = plan(w);
    assert_eq!(p.attempts().len(), 1);
    assert_eq!(p.attempts()[0].completed, true);
    assert_eq!(p.entries().len(), usize::from(expected == "case"));
    assert_eq!(p.attempts()[0].line, 2);
    if expected == "case" {
        assert_eq!(p.format("\n"), "case.feature:2");
    }
    if w.status == "SKIPPED" {
        assert_eq!(p.run_id(), None);
    }
    if w.status == "UNKNOWN" {
        assert_eq!(p.attempts()[0].status, Status::Unknown);
    }
}
#[then(
    "the formatted result agrees with the pinned JavaScript rerun formatter"
)]
fn official(w: &mut Acceptance) {
    let expected = oracle(w, "\n");
    let p = plan(w);
    assert_eq!(expected["formatted"], p.format("\n"));
    assert_eq!(expected["logged"], json!([p.format("\n")]));
    assert_eq!(
        expected["attempts"].as_array().unwrap().len(),
        p.attempts().len()
    );
    for (native, official) in
        p.attempts().iter().zip(expected["attempts"].as_array().unwrap())
    {
        assert_eq!(native.attempt, official["attempt"].as_u64().unwrap());
        assert_eq!(
            format!("{:?}", native.status).to_uppercase(),
            official["status"]
        );
        assert_eq!(
            native.will_be_retried,
            official["willBeRetried"].as_bool().unwrap()
        );
        assert_eq!(native.pickle_id, official["pickleId"]);
    }
}
#[then("every original retry attempt remains inspectable")]
fn history(w: &mut Acceptance) {
    let p = plan(w);
    assert_eq!(p.attempts().len(), 4);
    assert_eq!(p.attempts().iter().filter(|a| a.will_be_retried).count(), 2);
    assert_eq!(
        p.attempts().iter().filter(|a| a.status == Status::Failed).count(),
        3
    );
    assert_eq!(
        w.messages
            .iter()
            .filter(|e| e.get("testCaseStarted").is_some())
            .count(),
        4
    );
    assert!(
        w.messages
            .iter()
            .filter_map(|e| e.get("testStepFinished"))
            .any(|e| e["testStepResult"]["status"] == "FAILED"
                && e["testStepId"].as_str().unwrap().contains("after")),
        "actual cleanup failure must drive selection"
    );
}
#[then("only the exhausted terminal attempt contributes a rerun location")]
fn exhausted(w: &mut Acceptance) {
    let p = plan(w);
    assert_eq!(p.entries().len(), 1);
    assert_eq!(p.entries()[0].lines, vec![4]);
    let selected: Vec<_> =
        p.attempts().iter().filter(|a| a.is_selected()).collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].attempt, 1);
    assert!(!p.run_success());
}
#[when("the application executes the safe local rerun selection")]
async fn second(w: &mut Acceptance) {
    let selection = plan(w)
        .local_selection(|uri| Ok::<_, String>(PathBuf::from(uri)))
        .unwrap()
        .unwrap();
    let (sources, lines) = selection.into_parts();
    RECOVER.store(true, Ordering::SeqCst);
    EXECUTED.lock().unwrap().clear();
    let mut output = Vec::new();
    w.launched = true;
    Application::cucumber()
        .steps(steps())
        .with_gherkin43()
        .unwrap()
        .with_source_paths(sources)
        .with_line_selectors(lines)
        .with_messages(&mut output)
        .with_default_cli()
        .run("IGNORED_DEFAULT_PATH")
        .await;
    w.second = parse(output);
}
#[then("only the original failed Examples row locations execute again")]
fn exact_rows(w: &mut Acceptance) {
    let p = plan(w);
    assert_eq!(p.attempts().len(), 3);
    assert_eq!(p.entries()[0].lines, vec![7, 9]);
    let mut collector = Collector::new();
    for envelope in &w.second {
        collector.ingest(envelope).unwrap();
    }
    let second = collector.plan().unwrap();
    assert_eq!(second.attempts().len(), 2);
    let mut lines: Vec<_> = second.attempts().iter().map(|a| a.line).collect();
    lines.sort_unstable();
    assert_eq!(lines, vec![7, 9]);
    assert!(second.attempts().iter().all(|a| a.status == Status::Passed));
    assert!(second.entries().is_empty());
    assert!(second.run_success());
    assert_eq!(*EXECUTED.lock().unwrap(), vec!["fail", "fail"]);
}
#[then("the original attempt denominator and evidence remain unchanged")]
fn original(w: &mut Acceptance) {
    assert_eq!(w.messages, w.original);
    assert_eq!(plan(w).attempts().len(), 3);
    assert!(!plan(w).run_success());
    assert_eq!(
        plan(w).attempts().iter().filter(|a| a.is_selected()).count(),
        2
    );
}
#[then("URI groups and repeated lines preserve the official encounter order")]
fn grouped(w: &mut Acceptance) {
    assert_eq!(plan(w).format("\n"), "1:2\n2:2\nz.feature:2:2\na.feature:2:2");
    assert_eq!(plan(w).attempts().len(), 6);
}
#[then("a configured separator agrees with the pinned JavaScript formatter")]
fn separator(w: &mut Acceptance) {
    assert_eq!(oracle(w, " || ")["formatted"], plan(w).format(" || "));
}
#[then("the unfinished attempt is retained as UNKNOWN and selected")]
fn unknown(w: &mut Acceptance) {
    let p = plan(w);
    assert_eq!(p.attempts().len(), 1);
    assert!(!p.attempts()[0].completed);
    assert_eq!(p.attempts()[0].status, Status::Unknown);
    assert!(p.attempts()[0].is_selected());
    assert_eq!(p.format("\n"), "case.feature:2");
}
#[then(
    "collection reports an incomplete run rather than a successful empty plan"
)]
fn no_finish(w: &mut Acceptance) {
    assert_eq!(w.error, Some(Error::IncompleteRun));
    assert!(w.plan.is_none());
}
#[then(
    expr = "collection reports the missing {string} reference without guessed locations"
)]
fn bad_reference(w: &mut Acceptance, expected: String) {
    assert!(
        matches!(&w.error,Some(Error::MissingReference{kind,..}) if *kind==expected)
    );
    assert!(w.plan.is_none());
}
#[then("run failure remains visible and no scenario selection is invented")]
fn run_only(w: &mut Acceptance) {
    let p = plan(w);
    assert!(!p.run_success());
    assert!(p.entries().is_empty());
    assert!(p.attempts().is_empty());
    assert!(
        p.local_selection(|_| -> Result<PathBuf, String> {
            panic!("no URI may be invented")
        })
        .unwrap()
        .is_none()
    );
}
#[then("local selection is None and the URI resolver is never called")]
fn no_selection(w: &mut Acceptance) {
    let p = plan(w);
    assert!(p.run_success());
    assert_eq!(p.attempts().len(), 2);
    assert!(
        p.local_selection(|_| -> Result<PathBuf, String> {
            panic!("empty plan must not resolve any path")
        })
        .unwrap()
        .is_none()
    );
}
#[then("no unrestricted LineSelectors are used to launch a second run")]
fn no_run(w: &mut Acceptance) {
    assert!(!w.launched);
    assert!(w.second.is_empty());
    official(w);
}
#[then("a rejecting URI resolver returns its error without a fallback path")]
fn rejected(w: &mut Acceptance) {
    let error = plan(w)
        .local_selection(|_| {
            Err::<PathBuf, _>("not an admitted local resource")
        })
        .unwrap_err();
    assert_eq!(
        error,
        Error::Resolve {
            uri: "case.feature".into(),
            message: "not an admitted local resource".into()
        }
    );
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_rerun")
        .await;
}
