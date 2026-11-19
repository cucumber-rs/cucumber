//! Phase32a BDD: true original expressions and observed raw capture layouts.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    collections::BTreeSet, convert::Infallible, io::Write as _,
    panic::AssertUnwindSafe, process::Stdio, str::FromStr,
};

use cucumber::{
    World as _, given,
    step::{Collection, Context, DefinitionOptions},
    then, when,
};
use futures::{FutureExt as _, future::LocalBoxFuture};
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};

#[derive(Debug, Default, cucumber::World)]
struct Report {
    fixture: String,
    output: Vec<u8>,
    messages: Vec<Value>,
    oracle: Value,
    failed: bool,
    failure_message: String,
}
#[derive(Debug, Default, cucumber::World)]
struct Application;
#[given(expr_js = "paid {int}")]
fn paid(_: &mut Application, _value: i64) {}
#[given(expr_js = "rate {float}")]
fn rate(_: &mut Application, _value: f64) {}
#[given(expr_js = "word {word}")]
fn word(_: &mut Application, _value: String) {}
#[given(expr_js = "rest {}")]
fn rest(_: &mut Application, _value: String) {}
#[given(expr_js = "name {string}")]
fn name(_: &mut Application, _value: String) {}
#[given(expr_js = "😀 café {string}")]
fn unicode(_: &mut Application, _value: String) {}
#[given(expr_js = "belly/stomach holds {int} cucumber(s)")]
fn optional(_: &mut Application, _value: i64) {}
#[given(expr_js = "large {long}")]
fn large(_: &mut Application, _value: i64) {}
#[given(expr_js = "ambiguous {int}")]
fn ambiguous_int(_: &mut Application, _value: i64) {}
#[given(expr_js = "ambiguous {word}")]
fn ambiguous_word(_: &mut Application, _value: String) {}
#[given(expr = "legacy {int}")]
fn legacy(_: &mut Application, _value: i64) {}
#[derive(Debug, cucumber::Parameter)]
#[param(name = "color", regex = "red|blue")]
struct Color;
impl FromStr for Color {
    type Err = Infallible;
    fn from_str(_: &str) -> Result<Self, Self::Err> {
        Ok(Self)
    }
}
#[given(expr_js = "color {color}")]
fn custom(_: &mut Application, _value: Color) {}
fn action(_: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}
struct Manual;
impl cucumber::codegen::StepConstructor<Application> for Manual {
    fn inner(
        &self,
    ) -> (
        cucumber::step::Location,
        cucumber::codegen::LazyRegex,
        cucumber::Step<Application>,
    ) {
        (
            cucumber::step::Location {
                path: file!(),
                line: line!(),
                column: column!(),
            },
            || Regex::new("^manual$").unwrap(),
            action,
        )
    }
}
fn source(fixture: &str) -> (&str, &str) {
    match fixture {
        "integer" => ("paid {int}", "paid -42"),
        "float" => ("rate {float}", "rate +1.5E2"),
        "word" => ("word {word}", "word 東京"),
        "anonymous" => ("rest {}", "rest 😀 café"),
        "string-double" => ("name {string}", r#"name "Ada""#),
        "string-single" => ("name {string}", "name 'Ada'"),
        "string-empty" => ("name {string}", "name \"\""),
        "string-escaped" => ("name {string}", r#"name "a \"quoted\" word""#),
        "unicode-prefix" => ("😀 café {string}", r#"😀 café "é\"好""#),
        "optional-alternative" => (
            "belly/stomach holds {int} cucumber(s)",
            "stomach holds 2 cucumbers",
        ),
        "numeric-alias" => ("large {long}", "large -1234567890"),
        "ambiguous" => ("", "ambiguous 42"),
        "replacement" | "mismatch" => ("", "ready"),
        "legacy expression" => ("", "legacy 42"),
        "unannotated nested regex" => ("", "nested 42"),
        "external regex flags" => ("", "READY"),
        "undescribed custom type" => ("", "color blue"),
        "manual" => ("", "manual"),
        _ => panic!("unknown fixture {fixture}"),
    }
}
#[given(expr = "a metadata run using the {string} built-in expression")]
fn builtin(report: &mut Report, fixture: String) {
    report.fixture = fixture;
}
#[given(expr = "a metadata run using {string} registration")]
fn other(report: &mut Report, fixture: String) {
    report.fixture = fixture;
}
#[given("a metadata run with two matching built-in expressions")]
fn ambiguous(report: &mut Report) {
    report.fixture = "ambiguous".into();
}
#[given(
    "an expression registration replaced through the existing raw insertion \
        API"
)]
fn replacement(report: &mut Report) {
    report.fixture = "replacement".into();
}
#[given(
    "a raw registration carrying metadata that describes a different \
        expression"
)]
fn mismatch(report: &mut Report) {
    report.fixture = "mismatch".into();
}
#[given(
    "a manual StepConstructor implementing only the original required method"
)]
fn manual(report: &mut Report) {
    report.fixture = "manual".into();
}

async fn execute(report: &mut Report) {
    let mut steps = Application::collection();
    match report.fixture.as_str() {
        "replacement" | "mismatch" => {
            let snapshot = steps.support_code();
            let metadata = snapshot
                .definitions()
                .iter()
                .filter_map(|d| d.expression_metadata())
                .find(|m| m.source() == "paid {int}")
                .unwrap()
                .clone();
            steps = Collection::new().given_with_options(
                None,
                Regex::new("^ready$").unwrap(),
                action,
                DefinitionOptions::new().expression(metadata),
            );
            if report.fixture == "replacement" {
                steps =
                    steps.given(None, Regex::new("^ready$").unwrap(), action);
            }
        }
        "unannotated nested regex" => {
            steps = Collection::new().given(
                None,
                Regex::new(r"^nested ((\d)(\d))$").unwrap(),
                action,
            );
        }
        "external regex flags" => {
            steps = Collection::new().given(
                None,
                RegexBuilder::new("^ready$")
                    .case_insensitive(true)
                    .build()
                    .unwrap(),
                action,
            );
        }
        "manual" => {
            use cucumber::codegen::StepConstructor as _;
            let manual = Manual;
            let (location, regex, function) = manual.inner();
            steps = Collection::new().given_with_options(
                Some(location),
                regex(),
                function,
                manual.options(),
            );
        }
        _ => {}
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("expression-metadata.feature");
    std::fs::write(
        &path,
        format!(
            "Feature: Parameters\n  Scenario: Reported\n    Given {}\n",
            source(&report.fixture).1
        ),
    )
    .unwrap();
    let outcome = AssertUnwindSafe(
        Application::cucumber()
            .steps(steps)
            .with_messages(&mut report.output)
            .with_default_cli()
            .run(&path),
    )
    .catch_unwind()
    .await;
    report.failed = outcome.is_err();
    if let Err(error) = outcome {
        report.failure_message = error
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| error.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_default();
    }
    report.messages = String::from_utf8(report.output.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
}
#[when(
    "the run completes and the pinned JavaScript expression library reads \
        its matches"
)]
async fn run(report: &mut Report) {
    execute(report).await;
    assert!(!report.failed, "metadata run failed");
    let mut process = oracle::command("tests/interop/expressions/metadata.mjs")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(json!({"messages":report.messages}).to_string().as_bytes())
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "official expression consumer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    report.oracle = serde_json::from_slice(&output.stdout).unwrap();
}
#[when("that inconsistent registration executes")]
#[when("its collection executes with Messages reporting")]
async fn run_without_oracle(report: &mut Report) {
    execute(report).await;
}
fn step(report: &Report) -> &Value {
    &report.messages.iter().find_map(|m| m.get("testCase")).unwrap()["testStep\
        s"][0]
}
fn definitions(report: &Report) -> Vec<&Value> {
    report.messages.iter().filter_map(|m| m.get("stepDefinition")).collect()
}
fn assert_arguments(report: &Report) {
    let step = step(report);
    let ids = step["stepDefinitionIds"].as_array().unwrap();
    let lists = step["stepMatchArgumentsLists"].as_array().unwrap();
    let matches = report.oracle["cases"][0]["matches"].as_array().unwrap();
    assert_eq!(lists.len(), ids.len());
    assert_eq!(matches.len(), ids.len());
    for (index, expected) in matches.iter().enumerate() {
        assert_eq!(expected["id"], ids[index]);
        assert_eq!(lists[index]["stepMatchArguments"], expected["arguments"]);
    }
}
#[then(
    "its original expression and full raw argument groups agree with the \
        official library"
)]
fn original(report: &mut Report) {
    let id = &step(report)["stepDefinitionIds"][0];
    let definition =
        definitions(report).into_iter().find(|d| &d["id"] == id).unwrap();
    assert_eq!(
        definition["pattern"],
        json!({"type":"CUCUMBER_EXPRESSION","source":source(&report.fixture).0})
    );
    assert_arguments(report);
}
#[then(
    "native JavaScript indices verify the truthful range and isolate the \
        pinned nested-start discrepancy"
)]
fn repeated_capture(report: &mut Report) {
    let actual =
        &step(report)["stepMatchArgumentsLists"][0]["stepMatchArguments"];
    let expected = &report.oracle["cases"][0]["matches"][0];
    assert_eq!(actual, &expected["nativeArguments"]);
    let nested = &actual[0]["group"]["children"][0]["children"][0];
    let pinned =
        &expected["arguments"][0]["group"]["children"][0]["children"][0];
    assert_eq!(nested["start"], 16);
    assert_eq!(pinned["start"], 8);
    assert_eq!(nested["value"], pinned["value"]);
    let value = nested["value"].as_str().unwrap();
    let text = source("string-escaped").1;
    assert_eq!(&text[16..16 + value.len()], value);
    assert_ne!(&text[8..8 + value.len()], value);
    // Isolate exactly the demonstrated incorrect nested start. Every other
    // group, raw value, parameter name, presence and position still compares.
    let mut corrected = expected["arguments"].clone();
    corrected[0]["group"]["children"][0]["children"][0]["start"] = json!(16);
    assert_eq!(actual, &corrected);
}

#[then(
    "each reported argument list belongs to the definition at the same index"
)]
fn aligned(report: &mut Report) {
    assert_eq!(step(report)["stepDefinitionIds"].as_array().unwrap().len(), 2);
    assert_arguments(report);
}
#[then("only the actual used built-in parameter descriptors are emitted")]
fn parameters(report: &mut Report) {
    let parameters = report
        .messages
        .iter()
        .filter_map(|m| m.get("parameterType"))
        .collect::<Vec<_>>();
    let snapshot = Application::collection().support_code();
    let names = snapshot
        .definitions()
        .iter()
        .filter_map(|d| d.expression_metadata())
        .flat_map(|m| m.parameters().iter().map(|p| p.name.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        parameters
            .iter()
            .map(|p| p["name"].as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>(),
        names
    );
    assert_eq!(parameters.len(), names.len());
    for (parameter, expected) in
        parameters.iter().zip(report.oracle["parameters"].as_array().unwrap())
    {
        for key in [
            "name",
            "regularExpressions",
            "preferForRegularExpressionMatch",
            "useForSnippets",
        ] {
            assert_eq!(parameter[key], expected[key], "descriptor field {key}");
        }
        assert!(parameter.get("sourceReference").is_none());
    }
}
#[then(
    "only the raw regex remains and unavailable capture metadata is omitted"
)]
fn omitted(report: &mut Report) {
    let step = step(report);
    let id = &step["stepDefinitionIds"][0];
    let definition =
        definitions(report).into_iter().find(|d| &d["id"] == id).unwrap();
    assert_eq!(definition["pattern"]["type"], "REGULAR_EXPRESSION");
    assert!(step.get("stepMatchArgumentsLists").is_none());
    assert!(
        report
            .messages
            .iter()
            .any(|m| m["testStepFinished"]["testStepResult"]["status"]
                == "PASSED")
    );
}
#[then(
    "reporting identifies the metadata mismatch without a successful \
        run-finished envelope"
)]
fn mismatch_visible(report: &mut Report) {
    assert!(report.failed);
    assert!(
        report.failure_message.contains("expression metadata"),
        "{}",
        report.failure_message
    );
    assert!(
        !report
            .messages
            .iter()
            .any(|m| m["testRunFinished"]["success"] == true)
    );
}
#[then("its step still passes with raw definition metadata")]
fn old_constructor(report: &mut Report) {
    assert!(!report.failed);
    omitted(report);
}
#[then("the argument envelopes satisfy both supported Messages schemas")]
fn schema(report: &mut Report) {
    for source in [
        include_str!("fixtures/messages/messages-34.2.1.schema.json"),
        include_str!("fixtures/messages/messages-35.0.0.schema.json"),
    ] {
        let schema = serde_json::from_str(source).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for message in &report.messages {
            let errors = validator
                .iter_errors(message)
                .map(|e| e.to_string())
                .collect::<Vec<_>>();
            assert!(errors.is_empty(), "{message}: {errors:?}");
        }
    }
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_expression_metadata")
        .await;
}
