//! Phase-end BDD acceptance for the bounded Messages 34.2.1 writer.
use std::{
    collections::{HashMap, HashSet},
    panic::AssertUnwindSafe,
    sync::LazyLock,
};

use cucumber::{
    World as _, given,
    step::{Collection, Context, Matching},
    then, when,
    writer::Messages,
};
use futures::{FutureExt as _, future::LocalBoxFuture};
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default, cucumber::World)]
struct ReportWorld {
    kind: String,
    source: String,
    uri: String,
    output: String,
    error: Option<String>,
}

#[derive(Debug, Default, cucumber::World)]
struct RunWorld;

struct RecordedParser(cucumber::gherkin::Feature);

impl<I> cucumber::Parser<I> for RecordedParser {
    type Cli = cucumber::cli::Empty;
    type Output = futures::stream::Once<
        std::future::Ready<
            cucumber::parser::Result<cucumber::gherkin::Feature>,
        >,
    >;

    fn parse(self, _: I, _: Self::Cli) -> Self::Output {
        futures::stream::once(std::future::ready(Ok(self.0)))
    }
}

fn pass(_: &mut RunWorld, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}

fn fail(_: &mut RunWorld, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async { panic!("deliberate acceptance failure") })
}

#[given(expr = "a {string} plain English feature run")]
#[given(expr = "a {string} unsupported feature run")]
fn source(world: &mut ReportWorld, kind: String) {
    let body = match kind.as_str() {
        "passing" | "hooks" | "retries" | "missing source"
        | "changed source" => {
            "  Scenario: invoice\n    Given accepted\n    Then accepted\n"
        }
        "failing" => {
            "  Scenario: invoice\n    Given rejected\n    Then accepted\n"
        }
        "undefined" => {
            "  Scenario: invoice\n    Given unknown\n    Then accepted\n"
        }
        "ambiguous" => {
            "  Scenario: invoice\n    Given duplicate\n    Then accepted\n"
        }
        "background" => {
            "  Background: account\n    Given accepted\n  Scenario: first \
                invoice\n    Then accepted\n  Scenario: second invoice\n    \
                Then accepted\n"
        }
        "parallel" => {
            "\
                \x20 Scenario: first invoice\n\
                \x20   Given accepted\n\
                \x20   Then accepted\n\
                \x20 Scenario: second invoice\n\
                \x20   Given accepted\n\
                \x20   Then accepted\n"
        }
        "initial conjunction" => {
            "  Scenario: invoice\n    And accepted\n    But accepted\n"
        }
        "wildcard" => {
            "  Scenario: invoice\n    Given accepted\n    * accepted\n    \
                And accepted\n"
        }
        "inherited background" => {
            "\
                \x20 Background: account\n\
                \x20   When accepted\n\
                \x20 Scenario: invoice\n\
                \x20   And accepted\n"
        }
        "empty with background" => {
            "  Background: account\n    Given accepted\n  Scenario: invoice\n"
        }
        "empty" => "  Scenario: nothing to execute\n",
        "outline" => {
            "\
                \x20 Scenario Outline: invoice\n\
                \x20   Given <value>\n\
                \x20   Examples:\n\
                \x20     | value |\n\
                \x20     | accepted |\n"
        }
        "table" => {
            "\
                \x20 Scenario: invoice\n\
                \x20   Given accepted\n\
                \x20     | amount |\n\
                \x20     | 42 |\n"
        }
        "docstring" => {
            "  Scenario: invoice\n    Given accepted\n      \"\"\"\n      \
                invoice\n      \"\"\"\n"
        }
        "rule" => {
            "  Rule: invoice policy\n    Scenario: invoice\n      Given \
                accepted\n"
        }
        "tags" => "  @invoice\n  Scenario: invoice\n    Given accepted\n",
        "description" => {
            "  Extra explanation\n  Scenario: invoice\n    Given accepted\n"
        }
        "parse error" => "This is not a valid feature source\n",
        "language" => {
            "\
                # language: fr\n\
                Fonctionnalité: Factures\n\
                \x20 Scénario: facture\n\
                \x20   Soit accepted\n"
        }
        _ => panic!("Unknown acceptance run: {kind}"),
    };
    world.source = if matches!(kind.as_str(), "language" | "parse error") {
        body.to_owned()
    } else {
        format!("Feature: Shared invoice reports\n{body}")
    };
    world.kind = kind;
}

#[when("the run is reported as Cucumber Messages")]
async fn run(world: &mut ReportWorld) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invoice.feature");
    std::fs::write(&path, &world.source).unwrap();
    // The standard parser canonicalizes source paths.
    world.uri = path.canonicalize().unwrap().to_str().unwrap().to_owned();
    let recorded =
        if matches!(world.kind.as_str(), "missing source" | "changed source") {
            let feature = cucumber::gherkin::Feature::parse_path(
                &world.uri,
                Default::default(),
            )
            .unwrap();
            if world.kind == "missing source" {
                std::fs::remove_file(&path).unwrap();
            } else {
                std::fs::write(
                    &path,
                    world
                        .source
                        .replace("Shared invoice reports", "Changed source"),
                )
                .unwrap();
            }
            Some(feature)
        } else {
            None
        };
    let definitions = Collection::new()
        .given(None, Regex::new("^accepted$").unwrap(), pass)
        .given(None, Regex::new("^rejected$").unwrap(), fail)
        .given(None, Regex::new("^duplicate$").unwrap(), pass)
        .when(None, Regex::new("^duplicate$").unwrap(), fail);
    let mut output = Vec::new();
    let execution = async {
        let runner = RunWorld::cucumber()
            .steps(definitions)
            .step_matching(Matching::KeywordIndependent)
            .retries((world.kind == "retries").then_some(1))
            .max_concurrent_scenarios(4)
            .with_writer(Messages::new(&mut output))
            .with_default_cli();
        if let Some(feature) = recorded {
            runner
                .with_parser(RecordedParser(feature))
                .with_default_cli()
                .run(&path)
                .await;
        } else if world.kind == "hooks" {
            runner.before(|_, _, _, _| Box::pin(async {})).run(&path).await;
        } else {
            runner.run(&path).await;
        }
    };
    let result = AssertUnwindSafe(execution).catch_unwind().await;
    world.error = result.err().map(|error| {
        error
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| error.downcast_ref::<&str>().map(|s| (*s).to_owned()))
            .unwrap_or_else(|| "non-string panic".to_owned())
    });
    world.output = String::from_utf8(output).unwrap();
}

fn envelopes(world: &ReportWorld) -> Vec<Value> {
    world
        .output
        .lines()
        .map(|line| serde_json::from_str(line).expect("valid NDJSON line"))
        .collect()
}

#[then("every envelope validates against the official Messages 34.2.1 schema")]
fn schema(world: &mut ReportWorld) {
    assert!(
        world.error.is_none(),
        "unexpected reporting error: {:?}",
        world.error
    );
    assert!(world.output.ends_with('\n'));
    static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        let schema: Value = serde_json::from_str(include_str!(
            "fixtures/messages/messages-34.2.1.schema.json"
        ))
        .unwrap();
        jsonschema::validator_for(&schema)
            .expect("pinned official bundled schema compiles offline")
    });
    let messages = envelopes(world);
    assert!(!messages.is_empty());
    for envelope in messages {
        assert_eq!(
            envelope.as_object().unwrap().len(),
            1,
            "one message per envelope"
        );
        let errors = SCHEMA
            .iter_errors(&envelope)
            .map(|e| format!("{e} at {}", e.instance_path))
            .collect::<Vec<_>>();
        assert!(
            errors.is_empty(),
            "official schema rejected {envelope}: {errors:?}"
        );
    }
}

fn collect_ids(value: &Value, ids: &mut HashSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(id) = object.get("id").and_then(Value::as_str) {
                assert!(ids.insert(id.to_owned()), "duplicate ID: {id}");
            }
            for value in object.values() {
                collect_ids(value, ids);
            }
        }
        Value::Array(array) => {
            for value in array {
                collect_ids(value, ids);
            }
        }
        _ => {}
    }
}

fn assert_references(value: &Value, ids: &HashSet<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key.ends_with("Id") && key != "workerId" {
                    assert!(
                        ids.contains(value.as_str().unwrap()),
                        "dangling {key}: {value}"
                    );
                } else if key.ends_with("Ids") {
                    for id in value.as_array().unwrap() {
                        assert!(
                            ids.contains(id.as_str().unwrap()),
                            "dangling {key}: {id}"
                        );
                    }
                } else {
                    assert_references(value, ids);
                }
            }
        }
        Value::Array(array) => {
            for value in array {
                assert_references(value, ids);
            }
        }
        _ => {}
    }
}

fn nanos(value: &Value) -> u128 {
    u128::from(value["seconds"].as_u64().unwrap()) * 1_000_000_000
        + u128::from(value["nanos"].as_u64().unwrap())
}

#[then("source, AST, pickle and execution references are complete")]
fn references(world: &mut ReportWorld) {
    let messages = envelopes(world);
    let mut ids = HashSet::new();
    for message in &messages {
        collect_ids(message, &mut ids);
    }
    for message in &messages {
        assert_references(message, &ids);
    }
    assert_eq!(messages[0]["meta"]["protocolVersion"], "34.2.1");
    let source = messages.iter().find_map(|m| m.get("source")).unwrap();
    assert_eq!(source["uri"], world.uri);
    assert_eq!(source["data"], world.source);
    let ast = messages.iter().find_map(|m| m.get("gherkinDocument")).unwrap();
    assert_eq!(ast["uri"], world.uri);
    assert_eq!(ast["feature"]["name"], "Shared invoice reports");
    let ast_children = ast["feature"]["children"].as_array().unwrap();
    let scenario_count =
        ast_children.iter().filter(|c| c.get("scenario").is_some()).count();
    let pickles =
        messages.iter().filter_map(|m| m.get("pickle")).collect::<Vec<_>>();
    assert_eq!(pickles.len(), scenario_count);
    let cases =
        messages.iter().filter_map(|m| m.get("testCase")).collect::<Vec<_>>();
    assert_eq!(cases.len(), scenario_count);
    let mut started_steps = HashMap::new();
    let mut finished_steps = HashSet::new();
    let mut started_cases = HashSet::new();
    let mut finished_cases = HashSet::new();
    for message in &messages {
        if let Some(start) = message.get("testCaseStarted") {
            assert!(started_cases.insert(start["id"].as_str().unwrap()));
        }
        if let Some(start) = message.get("testStepStarted") {
            assert!(
                started_cases
                    .contains(start["testCaseStartedId"].as_str().unwrap())
            );
            assert!(
                started_steps
                    .insert(
                        start["testStepId"].as_str().unwrap(),
                        nanos(&start["timestamp"])
                    )
                    .is_none()
            );
        }
        if let Some(end) = message.get("testStepFinished") {
            let id = end["testStepId"].as_str().unwrap();
            assert!(finished_steps.insert(id));
            let elapsed = nanos(&end["timestamp"]) - started_steps[id];
            assert_eq!(elapsed, nanos(&end["testStepResult"]["duration"]));
        }
        if let Some(end) = message.get("testCaseFinished") {
            let id = end["testCaseStartedId"].as_str().unwrap();
            assert!(started_cases.contains(id));
            assert!(finished_cases.insert(id));
            assert_eq!(end["willBeRetried"], false);
        }
    }
    assert_eq!(started_cases, finished_cases);
    assert_eq!(started_steps.len(), finished_steps.len());
    assert_eq!(
        cases
            .iter()
            .map(|c| c["testSteps"].as_array().unwrap().len())
            .sum::<usize>(),
        finished_steps.len()
    );
    if world.kind == "background" {
        let background = &ast_children[0]["background"];
        let background_step = background["steps"][0]["id"].as_str().unwrap();
        assert_eq!(pickles.len(), 2);
        for pickle in pickles {
            assert_eq!(pickle["steps"][0]["astNodeIds"][0], background_step);
            assert_eq!(pickle["steps"][0]["text"], "accepted");
        }
    }
}

#[then(expr = "the pickle step types are {string}")]
fn pickle_types(world: &mut ReportWorld, expected: String) {
    let messages = envelopes(world);
    let types = messages
        .iter()
        .filter_map(|m| m.get("pickle"))
        .flat_map(|p| p["steps"].as_array().unwrap())
        .map(|s| s["type"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(types, expected);
}

#[then(expr = "the reported step statuses are {string}")]
fn statuses(world: &mut ReportWorld, expected: String) {
    let messages = envelopes(world);
    let statuses = messages
        .iter()
        .filter_map(|m| m.get("testStepFinished"))
        .map(|step| step["testStepResult"]["status"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(statuses, expected);
    if world.kind == "failing" {
        assert!(messages.iter().filter_map(|m| m.get("testStepFinished")).any(
            |s| {
                s["testStepResult"]["message"].as_str().is_some_and(|m| {
                    m.contains("deliberate acceptance failure")
                })
            }
        ));
    }
}

#[then(expr = "the completed run reports success {string}")]
fn completed(world: &mut ReportWorld, expected: String) {
    let messages = envelopes(world);
    let finished = messages
        .iter()
        .filter_map(|m| m.get("testRunFinished"))
        .collect::<Vec<_>>();
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0]["success"], expected.parse::<bool>().unwrap());
    assert!(messages.last().unwrap().get("testRunFinished").is_some());
}

#[then(expr = "reporting fails visibly with {string}")]
fn rejected(world: &mut ReportWorld, reason: String) {
    let error =
        world.error.as_ref().expect("unsupported report should fail visibly");
    assert!(error.contains(&reason), "expected {reason:?}, got {error:?}");
}

#[then("no successful completed run is emitted")]
fn no_success(world: &mut ReportWorld) {
    assert!(
        !envelopes(world)
            .iter()
            .filter_map(|m| m.get("testRunFinished"))
            .any(|finish| finish["success"] == true)
    );
}

#[tokio::main]
async fn main() {
    ReportWorld::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages")
        .await;
}
