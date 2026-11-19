//! Phase 13: differential BDD against the exact pinned official JS parser.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    collections::{HashMap, HashSet},
    io::Write as _,
    process::Stdio,
    sync::{LazyLock, Mutex},
};

use cucumber::{
    World as _, gherkin, given,
    step::{Collection, Context},
    then, when,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::{Value, json};

#[derive(Debug, Default, cucumber::World)]
struct Report {
    source: String,
    messages: Vec<Value>,
    official: Value,
    native_rejected: bool,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution;
static PAYLOADS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(Default::default);
fn record(_: &mut Execution, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        PAYLOADS
            .lock()
            .unwrap()
            .push(context.step.docstring.expect("handler received doc string"));
    })
}
#[given(
    expr = "a {string} doc-string using {string} syntax and {string} line \
        endings"
)]
fn configure(report: &mut Report, kind: String, edge: String, newline: String) {
    let delimiter = if kind == "quotes" { "\"\"\"" } else { "```" };
    let active = if kind == "quotes" { "\\\"\\\"\\\"" } else { "\\`\\`\\`" };
    let inactive = if kind == "quotes" { "\\`\\`\\`" } else { "\\\"\\\"\\\"" };
    let indent = match edge.as_str() {
        "NBSP indentation" => "  \u{a0}\u{a0}",
        "mixed Unicode indent" => " \u{a0}\t\u{2003}\u{3000}",
        "BOM indentation" => "\u{feff}\u{feff}",
        "all whitespace" => {
            "\t\u{b}\u{c}\r \u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\
                \u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\
                \u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}"
        }
        _ => "    ",
    };
    let body = match edge.as_str() {
        "repeated escapes" => format!(
            "{indent}value {active} and {active}\n{indent}other {inactive}\n"
        ),
        "inactive escapes" => format!(
            "{indent}{inactive} then {inactive}\n{indent}content {active}\n"
        ),
        "short indentation" => "  less\n\nnone\n      deeper\n".to_owned(),
        "whitespace boundary" => format!(
            "{indent}\u{85}NEL stays\n{indent}\u{200b}zero-width stays\n"
        ),
        _ => format!(
            "{indent}payload 🦀\n{indent}  deeper\n{indent}\n{indent}last\n"
        ),
    };
    let suffix = if edge == "closing suffix" {
        "ignored closing media text"
    } else {
        ""
    };
    report.source = format!(
        "Feature: Exact doc strings\n  Scenario: reusable payload\n    Given \
            payload\n{indent}{delimiter}\u{feff}application/json\u{a0}\n{bod\
            y}{indent}{delimiter}{suffix}\n"
    );
    if newline == "CRLF" {
        report.source = report.source.replace('\n', "\r\n");
    }
}
#[given("a legacy doc-string containing repeated escaped quotes")]
fn legacy_source(report: &mut Report) {
    report.source = "Feature: Legacy payload\n  Scenario: untouched native \
        representation\n    Given payload\n      \"\"\"\n      value \
        \\\"\\\"\\\" and \\\"\\\"\\\"\n      \"\"\"\n"
        .into();
}
#[given("a doc-string delimiter indented with NBSP")]
fn nbsp_source(report: &mut Report) {
    configure(report, "quotes".into(), "NBSP indentation".into(), "LF".into());
}
fn oracle(source: &str) -> Value {
    let mut child = oracle::command("tests/interop/gherkin43/oracle.mjs")
        .args(["official\
        .feature"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect(
            "Node and npm ci --ignore-scripts in tests/interop/gherkin43 \
            are required",
        );
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "official parser failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
async fn execute(report: &mut Report, semantics: gherkin::DocStringSemantics) {
    report.official = oracle(&report.source);
    PAYLOADS.lock().unwrap().clear();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("docstrings.feature");
    std::fs::write(&path, &report.source).unwrap();
    let mut output = Vec::new();
    Execution::cucumber()
        .steps(Collection::new().given(
            None,
            Regex::new("^payload$").unwrap(),
            record,
        ))
        .with_docstring_semantics(semantics)
        .with_messages(&mut output)
        .with_default_cli()
        .run(&path)
        .await;
    report.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
}
#[when(
    "the feature runs with Gherkin 43 doc-string semantics and captured \
        Messages"
)]
async fn official_run(report: &mut Report) {
    execute(report, gherkin::DocStringSemantics::Gherkin43).await;
}
#[when(
    "the feature runs with legacy doc-string semantics and captured Messages"
)]
async fn legacy_run(report: &mut Report) {
    execute(report, gherkin::DocStringSemantics::Legacy).await;
}
#[when("it is parsed with the legacy native policy")]
fn parse_legacy(report: &mut Report) {
    report.native_rejected =
        gherkin::Feature::parse(&report.source, Default::default()).is_err();
}
#[then("the legacy parser still rejects that newly opt-in syntax")]
fn legacy_rejected(report: &mut Report) {
    assert!(report.native_rejected);
}
#[then("the actual handler payload equals the official Pickle content")]
fn payload(report: &mut Report) {
    let expected = report.official["pickles"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["steps"].as_array().unwrap())
        .map(|s| {
            s["argument"]["docString"]["content"].as_str().unwrap().to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(*PAYLOADS.lock().unwrap(), expected);
}
#[then("the handler retains the exact legacy payload")]
fn native_legacy(_: &mut Report) {
    assert_eq!(
        *PAYLOADS.lock().unwrap(),
        ["\nvalue \\\"\\\"\\\" and \\\"\\\"\\\"\n"]
    );
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
#[then(
    "the complete source AST and Pickles equal the pinned official parser \
        modulo IDs and URI"
)]
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
#[then("doc-string media type and delimiter location match the official AST")]
fn doc_location(report: &mut Report) {
    let ast =
        report.messages.iter().find_map(|m| m.get("gherkinDocument")).unwrap();
    let doc =
        &ast["feature"]["children"][0]["scenario"]["steps"][0]["docString"];
    let expected = &report.official["gherkinDocument"]["feature"]["children"]
        [0]["scenario"]["steps"][0]["docString"];
    assert_eq!(doc["mediaType"], "application/json");
    assert_eq!(doc["location"], expected["location"]);
}
#[then("every envelope satisfies Messages 34 and the run passed")]
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
    let results = report
        .messages
        .iter()
        .filter_map(|m| m.get("testStepFinished"))
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["testStepResult"]["status"], "PASSED");
    assert_eq!(
        report.messages.last().unwrap()["testRunFinished"]["success"],
        true
    );
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_docstrings")
        .await;
}
