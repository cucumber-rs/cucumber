//! Phase 10 end-of-phase acceptance for real native retry streams.
use std::{collections::{HashMap, HashSet}, sync::LazyLock};
use cucumber::{World as _, given, when, then, step::{Collection, Context, Matching}};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default, cucumber::World)]
struct Report { outcome: String, messages: Vec<Value> }
#[derive(Debug, Default, cucumber::World)]
struct Execution;
static COUNTS: LazyLock<std::sync::Mutex<HashMap<String, usize>>> = LazyLock::new(Default::default);
fn act(_: &mut Execution, context: Context) -> LocalBoxFuture<'_, ()> {
    let name = context.step.value.clone();
    let count = { let mut counts = COUNTS.lock().unwrap(); let count = counts.entry(name.clone()).or_default(); *count += 1; *count };
    Box::pin(async move { if name == "exhausted" || (name == "recovered" && count == 1) { panic!("deliberate attempt failure"); } })
}
fn noop(_: &mut Execution, _: Context) -> LocalBoxFuture<'_, ()> { Box::pin(async {}) }
#[given(expr = "a retrying feature with outcome {string}")]
fn set_outcome(report: &mut Report, outcome: String) { report.outcome = outcome; }
#[given("concurrent recovering and exhausted scenarios")]
fn concurrent(report: &mut Report) { report.outcome = "concurrent".into(); }
async fn execute(report: &mut Report, budget: usize) {
    COUNTS.lock().unwrap().clear();
    let names = if report.outcome == "concurrent" { vec!["recovered", "exhausted"] } else { vec![report.outcome.as_str()] };
    let mut source = "Feature: Retry reporting\n".to_owned();
    for name in names { source.push_str(&format!("  Scenario: {name}\n    Given {name}\n    Then completed\n")); }
    let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("retry.feature");
    std::fs::write(&path, source).unwrap();
    let steps = Collection::new().given(None, Regex::new("^(recovered|exhausted|passing)$").unwrap(), act)
        .then(None, Regex::new("^completed$").unwrap(), noop);
    let mut output = Vec::new();
    Execution::cucumber().steps(steps).step_matching(Matching::KeywordIndependent)
        .max_concurrent_scenarios(2).retries(budget).with_messages(&mut output).with_default_cli().run(&path).await;
    report.messages = String::from_utf8(output).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
}
#[when(expr = "it runs with a retry budget of {int} through the Messages writer")]
async fn run(report: &mut Report, budget: usize) { execute(report, budget).await; }
#[when("they run through the Messages writer")]
async fn run_concurrent(report: &mut Report) { execute(report, 1).await; }
fn messages<'a>(report: &'a Report, kind: &str) -> Vec<&'a Value> { report.messages.iter().filter_map(|m|m.get(kind)).collect() }
#[then(expr = "the report contains {int} distinct attempts of one test case")]
fn attempts(report: &mut Report, count: usize) {
    let cases = messages(report, "testCase"); assert_eq!(cases.len(), 1);
    let starts = messages(report, "testCaseStarted"); assert_eq!(starts.len(), count);
    let mut ids = HashSet::new();
    for (i, start) in starts.iter().enumerate() { assert_eq!(start["attempt"], i); assert_eq!(start["testCaseId"], cases[0]["id"]); assert!(ids.insert(start["id"].as_str().unwrap())); }
    let finishes = messages(report, "testCaseFinished"); assert_eq!(finishes.len(), count);
    for (start, finish) in starts.iter().zip(finishes) { assert_eq!(start["id"], finish["testCaseStartedId"]); }
}
#[then(expr = "attempt retry flags are {string}")]
fn flags(report: &mut Report, expected: String) {
    let flags = messages(report,"testCaseFinished").iter().map(|m| m["willBeRetried"].to_string()).collect::<Vec<_>>().join(","); assert_eq!(flags, expected);
}
#[then(expr = "attempt first-step statuses are {string}")]
fn statuses(report: &mut Report, expected: String) {
    let starts = messages(report,"testCaseStarted");
    let outcomes = messages(report,"testStepFinished");
    let statuses = starts.iter().map(|start| outcomes.iter().find(|m|m["testCaseStartedId"] == start["id"]).unwrap()["testStepResult"]["status"].as_str().unwrap()).collect::<Vec<_>>().join(",");
    assert_eq!(statuses, expected);
    for start in starts { assert_eq!(outcomes.iter().filter(|m|m["testCaseStartedId"] == start["id"]).count(), 2); }
}
#[then(expr = "the final run success is {string}")]
fn success(report: &mut Report, expected: String) { assert_eq!(messages(report,"testRunFinished")[0]["success"], expected == "true"); }
#[then("each scenario has its own contiguous attempt sequence")]
fn sequences(report: &mut Report) {
    let cases = messages(report,"testCase"); assert_eq!(cases.len(),2);
    let starts = messages(report,"testCaseStarted"); assert_eq!(starts.len(),4);
    for case in cases { let attempts = starts.iter().filter(|s|s["testCaseId"] == case["id"]).map(|s|s["attempt"].as_u64().unwrap()).collect::<Vec<_>>(); assert_eq!(attempts, [0,1]); }
}
#[then("the recovered scenario does not hide the exhausted failure")]
fn mixed(report: &mut Report) {
    success(report,"false".into());
    let pickles = messages(report,"pickle"); let cases = messages(report,"testCase"); let starts = messages(report,"testCaseStarted"); let outcomes=messages(report,"testStepFinished");
    for (name,status) in [("recovered","PASSED"),("exhausted","FAILED")] {
        let pickle = pickles.iter().find(|p|p["name"] == name).unwrap(); let case=cases.iter().find(|c|c["pickleId"] == pickle["id"]).unwrap(); let start=starts.iter().find(|s|s["testCaseId"] == case["id"] && s["attempt"] == 1).unwrap();
        assert_eq!(outcomes.iter().find(|s|s["testCaseStartedId"] == start["id"]).unwrap()["testStepResult"]["status"],status);
    }
}
fn ids(value: &Value, all: &mut HashSet<String>) {
    match value { Value::Object(map) => { if let Some(id)=map.get("id") { assert!(all.insert(id.as_str().unwrap().to_owned()),"duplicate ID {id}"); } for value in map.values() { ids(value, all); } }, Value::Array(values) => for value in values { ids(value,all); }, _=>{} }
}
fn references(value: &Value, all: &HashSet<String>) {
    match value { Value::Object(map) => for (key,value) in map { if key.ends_with("Id") { assert!(all.contains(value.as_str().unwrap()),"dangling {key} {value}"); } else if key.ends_with("Ids") { for id in value.as_array().unwrap() { assert!(all.contains(id.as_str().unwrap())); } } else { references(value,all); } }, Value::Array(values)=>for value in values { references(value,all); }, _=>{} }
}
#[then("all retry envelopes satisfy the pinned schema and reference graph")]
fn schema(report: &mut Report) {
    static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| jsonschema::validator_for(&serde_json::from_str(include_str!("fixtures/messages/messages-34.2.1.schema.json")).unwrap()).unwrap());
    let mut all = HashSet::new();
    for message in &report.messages { let errors=SCHEMA.iter_errors(message).map(|e|e.to_string()).collect::<Vec<_>>(); assert!(errors.is_empty(),"{message}: {errors:?}"); ids(message,&mut all); }
    for message in &report.messages { references(message,&all); }
    let mut active: HashMap<String,HashSet<String>>=HashMap::new();
    for message in &report.messages {
        if let Some(start)=message.get("testCaseStarted") { assert!(active.insert(start["id"].as_str().unwrap().into(),HashSet::new()).is_none()); }
        if let Some(start)=message.get("testStepStarted") { assert!(active.get_mut(start["testCaseStartedId"].as_str().unwrap()).unwrap().insert(start["testStepId"].as_str().unwrap().into())); }
        if let Some(end)=message.get("testStepFinished") { assert!(active.get_mut(end["testCaseStartedId"].as_str().unwrap()).unwrap().remove(end["testStepId"].as_str().unwrap())); }
        if let Some(end)=message.get("testCaseFinished") { assert!(active.remove(end["testCaseStartedId"].as_str().unwrap()).unwrap().is_empty()); }
    }
    assert!(active.is_empty());
}
#[tokio::main]
async fn main() { Report::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/messages_retries").await; }
