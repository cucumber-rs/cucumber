//! Phase 16: explicit empty-scenario background execution and official graphs.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    collections::{HashMap, HashSet},
    io::Write as _,
    panic::AssertUnwindSafe,
    process::Stdio,
    sync::{LazyLock, Mutex},
};

use cucumber::{
    Parser as _, StatsWriter as _, World as _, WriterExt as _, given,
    parser::{self, SourceStore},
    runner::{self, BackgroundSemantics, Hooks, ScenarioType},
    step::{Collection, Context, Matching},
    then, when,
    writer::{self, Coloring, messages::source::SourceGraph},
};
use futures::{FutureExt as _, StreamExt as _, future::LocalBoxFuture};
use regex::Regex;
use serde_json::{Value, json};
#[derive(Debug, Default, cucumber::World)]
struct Report {
    source: String,
    messages: Vec<Value>,
    official: Value,
    error: Option<String>,
    native_ok: bool,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution {
    name: String,
}
static TRACE: LazyLock<Mutex<Vec<(String, String)>>> =
    LazyLock::new(Default::default);
fn act(world: &mut Execution, ctx: Context) -> LocalBoxFuture<'_, ()> {
    let name = world.name.clone();
    Box::pin(async move {
        TRACE.lock().unwrap().push((name, ctx.step.value));
    })
}
fn steps() -> Collection<Execution> {
    Collection::new()
        .given(None, Regex::new(".*").unwrap(), act)
        .matching(Matching::KeywordIndependent)
}
fn hooks() -> Hooks<Execution> {
    Hooks::<Execution>::new()
        .before("setup", |_, _, scenario, world| {
            Box::pin(async move {
                world.name = scenario.name.clone();
                TRACE
                    .lock()
                    .unwrap()
                    .push((scenario.name.clone(), "before".into()));
            })
        })
        .after("cleanup", |_, _, scenario, _, _| {
            Box::pin(async move {
                TRACE
                    .lock()
                    .unwrap()
                    .push((scenario.name.clone(), "after".into()));
            })
        })
}
#[given(expr = "a {string} feature with background steps")]
fn configure(report: &mut Report, shape: String) {
    let feature = "\
        Feature: Empty scenarios\n\
        \x20 Background: feature setup\n\
        \x20   Given feature setup\n";
    let rule = "  Rule: per-account\n    Background: rule setup\n      Given \
        rule setup\n";
    let source = match shape.as_str() {
        "feature empty" => format!("{feature}  Scenario: empty feature\n"),
        "rule empty" => format!("{feature}{rule}    Scenario: empty rule\n"),
        "feature outline" => format!(
            "\
                {feature}  Scenario Outline: empty <id>\n\
                \x20   Examples: rows\n\
                \x20     | id |\n\
                \x20     | one |\n\
                \x20     | two |\n"
        ),
        "rule outline" => format!(
            "{feature}{rule}    Scenario Outline: empty <id>\n      \
                Examples: rows\n        | id |\n        | one |\n        | \
                two |\n"
        ),
        "mixed" => format!(
            "\
                {feature}  Scenario: empty feature\n\
                \x20 Scenario: filled feature\n\
                \x20   Given work\n\
                {rule}    Scenario: empty rule\n\
                \x20   Scenario: filled rule\n\
                \x20     Given work\n"
        ),
        "no background" => "Feature: No background\n  Scenario: empty\n".into(),
        "official fixture" => std::fs::read_to_string(
            "tests/fixtures/gherkin43/incomplete_scenario.feature",
        )
        .unwrap(),
        _ => panic!("unknown shape {shape}"),
    };
    report.source = source;
    TRACE.lock().unwrap().clear();
}
fn oracle(source: &str) -> Value {
    let mut child = oracle::command("tests/interop/gherkin43/oracle.mjs")
        .args(["official.feature"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pinned oracle installed");
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}
fn read_messages(report: &mut Report, output: Vec<u8>) {
    report.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
}
async fn official_run(report: &mut Report, clone: bool) {
    report.official = oracle(&report.source);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("backgrounds.feature");
    std::fs::write(&path, &report.source).unwrap();
    let mut output = Vec::new();
    if clone {
        // Every type-changing native builder follows policy selection. Cloning
        // also must retain it. with_hooks replaces the preceding single hooks.
        let configured = runner::Basic::<Execution>::default()
            .with_background_semantics(BackgroundSemantics::Gherkin43);
        let selected = configured
            .clone()
            .which_scenario(|_, _, _| ScenarioType::Serial)
            .before(|_, _, _, _| Box::pin(async {}))
            .after(|_, _, _, _, _| Box::pin(async {}))
            .with_hooks(hooks())
            .steps(steps());
        Execution::cucumber()
            .with_runner(selected)
            .with_messages(&mut output)
            .with_default_cli()
            .run(&path)
            .await;
    } else {
        Execution::cucumber()
            .with_background_semantics(BackgroundSemantics::Gherkin43)
            .with_hooks(hooks())
            .steps(steps())
            .max_concurrent_scenarios(1)
            .with_messages(&mut output)
            .with_default_cli()
            .run(&path)
            .await;
    }
    read_messages(report, output);
}
#[when("it executes with the Gherkin 43 background policy and Messages")]
async fn official_execution(report: &mut Report) {
    official_run(report, false).await;
}
#[when(
    "a cloned configured runner executes after hook and scheduling builders"
)]
async fn cloned_execution(report: &mut Report) {
    official_run(report, true).await;
}
#[when("it executes with the default runner and native summary writer")]
async fn legacy_execution(report: &mut Report) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.feature");
    std::fs::write(&path, &report.source).unwrap();
    let mut output = Vec::new();
    let result = Execution::cucumber()
        .with_hooks(hooks())
        .steps(steps())
        .max_concurrent_scenarios(1)
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .run(&path)
        .await;
    report.native_ok = result.failed_steps() == 0
        && result.parsing_errors() == 0
        && result.hook_errors() == 0
        && result.skipped_steps() == 0;
}
#[when("it executes with the default runner and source-backed Messages")]
async fn rejected_execution(report: &mut Report) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.feature");
    std::fs::write(&path, &report.source).unwrap();
    let mut output = Vec::new();
    let run = Execution::cucumber()
        .with_hooks(hooks())
        .steps(steps())
        .max_concurrent_scenarios(1)
        .with_messages(&mut output)
        .with_default_cli()
        .run(&path);
    let result = AssertUnwindSafe(run).catch_unwind().await;
    if let Err(error) = result {
        report.error = Some(
            error
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default(),
        );
    }
    read_messages(report, output);
}
#[when("its legacy parser snapshot is converted without execution")]
async fn direct_graph(report: &mut Report) {
    report.official = oracle(&report.source);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("graph.feature");
    std::fs::write(&path, &report.source).unwrap();
    let store = SourceStore::new();
    let parsed = parser::Basic::new()
        .with_source_store(store.clone())
        .parse(path.clone(), Default::default())
        .collect::<Vec<_>>()
        .await;
    assert_eq!(parsed.len(), 1);
    assert!(parsed[0].is_ok());
    let parsed_path = parsed[0].as_ref().unwrap().path.as_ref().unwrap();
    let snapshot = store
        .get(parsed_path)
        .expect("snapshot uses parser canonical source identity");
    let graph =
        SourceGraph::from_snapshot(parsed_path.to_str().unwrap(), &snapshot)
            .unwrap();
    report.messages = vec![json!({"gherkinDocument":graph.gherkin_document()})];
    report.messages.extend(graph.pickles().iter().map(|p| json!({"pickle":p})));
}
fn msgs<'a>(report: &'a Report, key: &str) -> Vec<&'a Value> {
    report.messages.iter().filter_map(|m| m.get(key)).collect()
}
#[then(expr = "real handlers ran {int} background steps and {int} body steps")]
fn handlers(_: &mut Report, backgrounds: usize, bodies: usize) {
    let trace = TRACE.lock().unwrap();
    assert_eq!(
        trace
            .iter()
            .filter(|(_, step)| matches!(
                step.as_str(),
                "feature setup" | "rule setup" | "a step"
            ))
            .count(),
        backgrounds
    );
    assert_eq!(trace.iter().filter(|(_, step)| step == "work").count(), bodies);
}
#[then("the native run has no failures or parser errors")]
fn native_ok(report: &mut Report) {
    assert!(report.native_ok);
}
#[then("reporting visibly rejects the unexpected background execution")]
fn rejected(report: &mut Report) {
    assert!(
        report
            .error
            .as_deref()
            .is_some_and(|e| e
                .contains("step does not match the next planned pickle step")),
        "{:?}",
        report.error
    );
}
#[then("no successful empty-scenario completion is emitted")]
fn no_success(report: &mut Report) {
    assert!(msgs(report, "testRunFinished").is_empty());
    assert!(msgs(report, "testCaseFinished").is_empty());
}
#[then("empty Pickles execute only their scenario hook stages")]
fn executions(report: &mut Report) {
    let pickles = msgs(report, "pickle");
    let cases = msgs(report, "testCase");
    let starts = msgs(report, "testCaseStarted");
    let ends = msgs(report, "testStepFinished");
    assert_eq!(pickles.len(), cases.len());
    assert_eq!(cases.len(), starts.len());
    for pickle in pickles {
        let case =
            cases.iter().find(|c| c["pickleId"] == pickle["id"]).unwrap();
        let steps = case["testSteps"].as_array().unwrap();
        let pickle_steps = pickle["steps"].as_array().unwrap();
        assert_eq!(steps.len(), pickle_steps.len() + 2);
        assert!(steps.first().unwrap().get("hookId").is_some());
        assert!(steps.last().unwrap().get("hookId").is_some());
        if pickle_steps.is_empty() {
            assert!(steps.iter().all(|s| s.get("pickleStepId").is_none()));
        }
        let attempt =
            starts.iter().find(|s| s["testCaseId"] == case["id"]).unwrap();
        assert_eq!(
            ends.iter()
                .filter(|e| e["testCaseStartedId"] == attempt["id"])
                .count(),
            steps.len()
        );
        let trace = TRACE.lock().unwrap();
        let name = pickle["name"].as_str().unwrap();
        let observed = trace
            .iter()
            .filter(|(scenario, _)| scenario == name)
            .map(|(_, step)| step.as_str())
            .collect::<Vec<_>>();
        assert_eq!(observed.first(), Some(&"before"));
        assert_eq!(observed.last(), Some(&"after"));
        if pickle_steps.is_empty() {
            assert_eq!(observed, ["before", "after"]);
        }
    }
}
#[then("its empty Pickles retain feature and Rule backgrounds only in the AST")]
fn source_only(report: &mut Report) {
    assert!(
        msgs(report, "pickle")
            .iter()
            .all(|p| p["steps"].as_array().unwrap().is_empty())
    );
    let ast = msgs(report, "gherkinDocument")[0];
    assert_eq!(
        ast["feature"]["children"][0]["background"]["steps"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        ast["feature"]["children"][1]["rule"]["children"][0]["backgrou\
        nd"]["steps"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(TRACE.lock().unwrap().is_empty());
}

fn collect_ids(
    expected: &Value,
    actual: &Value,
    ids: &mut HashMap<String, String>,
    inverse: &mut HashSet<String>,
) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            assert_eq!(
                e.keys().collect::<Vec<_>>(),
                a.keys().collect::<Vec<_>>(),
                "field set differs: {expected} / {actual}"
            );
            if let Some(id) = e.get("id") {
                assert!(
                    ids.insert(
                        a["id"].as_str().unwrap().into(),
                        id.as_str().unwrap().into()
                    )
                    .is_none()
                );
                assert!(inverse.insert(id.as_str().unwrap().into()));
            }
            for (key, value) in e {
                collect_ids(value, &a[key], ids, inverse);
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(e.len(), a.len());
            for (e, a) in e.iter().zip(a) {
                collect_ids(e, a, ids, inverse);
            }
        }
        _ => {}
    }
}
fn normalize(value: &mut Value, ids: &HashMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if key == "uri" {
                    *value = json!("official.feature");
                } else if key == "id" || key.ends_with("Id") {
                    *value = json!(ids[value.as_str().unwrap()]);
                } else if key.ends_with("Ids") {
                    for id in value.as_array_mut().unwrap() {
                        *id = json!(ids[id.as_str().unwrap()]);
                    }
                } else {
                    normalize(value, ids);
                }
            }
        }
        Value::Array(array) => {
            for item in array {
                normalize(item, ids);
            }
        }
        _ => {}
    }
}
#[then("its complete AST and Pickles match the pinned official compiler")]
fn graph(report: &mut Report) {
    let mut actual = json!({
    "gherkinDocument": report.messages.iter().find_map(|m|m.get("gherkinDocum\
        ent")).unwrap(),

    "pickles": report
        .messages
        .iter()
        .filter_map(|m|m
        .get("pickle"))
        .collect::<Vec<_>>()}
        );
    let mut ids = HashMap::new();
    collect_ids(&report.official, &actual, &mut ids, &mut HashSet::new());
    normalize(&mut actual, &ids);
    assert_eq!(
        actual, report.official,
        "no content, location, media, delimiter or linkage fields may be \
            dropped"
    );
}
#[then("the background report is schema-valid and successful")]
fn schema(report: &mut Report) {
    static VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        jsonschema::validator_for(
            &serde_json::from_str(include_str!(
                "fixtures/messages/messages-34.2.1.schema.json"
            ))
            .unwrap(),
        )
        .unwrap()
    });
    for message in &report.messages {
        let errors = VALIDATOR
            .iter_errors(message)
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
    assert!(
        msgs(report, "testStepFinished")
            .iter()
            .all(|s| s["testStepResult"]["status"] == "PASSED")
    );
    assert_eq!(msgs(report, "testRunFinished").len(), 1);
    assert_eq!(msgs(report, "testRunFinished")[0]["success"], true);
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_backgrounds")
        .await;
}
