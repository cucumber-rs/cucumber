//! Phase 17 behavior acceptance for error reporting from real native parsing.
use std::{path::PathBuf, sync::LazyLock};

use cucumber::{
    Parser as _, World as _, given,
    parser::{Basic, SourceStore},
    step::{Collection, Context},
    then, when,
    writer::Messages,
};
use futures::{StreamExt as _, future::LocalBoxFuture};
use regex::Regex;
use serde_json::Value;
#[derive(Debug, Default, cucumber::World)]
struct Report {
    kind: String,
    directory: Option<tempfile::TempDir>,
    path: PathBuf,
    source: String,
    store: SourceStore,
    messages: Vec<Value>,
    location: Option<(usize, usize)>,
    retained: Option<std::sync::Arc<cucumber::parser::SourceSnapshot>>,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution;
struct Recorded(Vec<cucumber::parser::Result<cucumber::gherkin::Feature>>);
impl<I> cucumber::Parser<I> for Recorded {
    type Cli = cucumber::cli::Empty;
    type Output = futures::stream::Iter<
        std::vec::IntoIter<
            cucumber::parser::Result<cucumber::gherkin::Feature>,
        >,
    >;
    fn parse(self, _: I, _: Self::Cli) -> Self::Output {
        futures::stream::iter(self.0)
    }
}
fn pass(_: &mut Execution, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}
#[given(expr = "an input with a {string} failure")]
fn input(world: &mut Report, kind: String) {
    world.kind = kind.clone();
    let directory = tempfile::tempdir().unwrap();
    world.path = directory.path().join("broken.feature");
    world.source = match kind.as_str() {
        "syntax" => "\
                Feature: Broken\n\
                \x20 Scenario: Input\n\
                \x20   Given accepted\n\
                \x20     | a | b |\n\
                \x20     | one |\n"
            .into(),

        "expansion" => "Feature: Broken\n  Scenario Outline: <missing>\n    \
            Given accepted\n    Examples:\n      | present |\n      | value \
            |\n"
        .into(),

        "missing file" => String::new(),
        _ => panic!("unknown failure"),
    };
    if kind != "missing file" {
        std::fs::write(&world.path, &world.source).unwrap();
        world.path = world.path.canonicalize().unwrap();
    }
    world.directory = Some(directory);
}
async fn record(
    world: &mut Report,
    results: Vec<cucumber::parser::Result<cucumber::gherkin::Feature>>,
) {
    let mut output = Vec::new();
    Execution::cucumber::<PathBuf>()
        .with_parser(Recorded(results))
        .steps(Collection::new().given(
            None,
            Regex::new("^accepted$").unwrap(),
            pass,
        ))
        .with_writer(Messages::with_source_store(
            &mut output,
            world.store.clone(),
        ))
        .with_default_cli()
        .run(&world.path)
        .await;
    world.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
}
#[when(
    expr = "parsing finishes and the source file is {string} before reporting"
)]
async fn mutate(world: &mut Report, change: String) {
    let results = Basic::new()
        .with_source_store(world.store.clone())
        .parse(&world.path, Default::default())
        .collect::<Vec<_>>()
        .await;
    assert_eq!(results.len(), 1);
    let error = results[0].as_ref().unwrap_err();
    world.location = Some(match error {
        cucumber::parser::Error::Parsing(error) => match error.as_ref() {
            cucumber::gherkin::ParseFileError::Parsing { source, .. } => {
                (source.position().line, source.position().col)
            }
            _ => panic!("not syntax"),
        },
        cucumber::parser::Error::ExampleExpansion(error) => {
            (error.pos.line, error.pos.col)
        }
    });
    if change == "deleted" {
        std::fs::remove_file(&world.path).unwrap();
    } else {
        std::fs::write(&world.path, "Feature: Rewritten\n").unwrap();
    }
    record(world, results).await;
}
#[given("a successful source snapshot retained before its file is deleted")]
async fn generation(world: &mut Report) {
    input(world, "missing file".into());
    world.source =
        "Feature: Retained\n  Scenario: Old\n    Given accepted\n".into();
    std::fs::write(&world.path, &world.source).unwrap();
    world.path = world.path.canonicalize().unwrap();
    let parsed = Basic::new()
        .with_source_store(world.store.clone())
        .parse(&world.path, Default::default())
        .collect::<Vec<_>>()
        .await;
    assert!(parsed[0].is_ok());
    world.retained = world.store.get(&world.path);
    std::fs::remove_file(&world.path).unwrap();
}
#[then(
    "the current input is cleared and the retained snapshot stays unchanged"
)]
fn old_generation(world: &mut Report) {
    assert!(world.store.input(&world.path).is_none());
    assert!(world.store.get(&world.path).is_none());
    assert_eq!(world.retained.as_ref().unwrap().source(), world.source);
}
#[when("the missing input is reported")]
async fn missing(world: &mut Report) {
    let results = Basic::new()
        .with_source_store(world.store.clone())
        .parse(&world.path, Default::default())
        .collect::<Vec<_>>()
        .await;
    record(world, results).await;
}
#[given("a directory with valid and invalid feature inputs")]
fn mixed(world: &mut Report) {
    input(world, "syntax".into());
    std::fs::write(
        world.directory.as_ref().unwrap().path().join("valid.feature"),
        "Feature: Valid\n  Scenario: Accepted\n    Given accepted\n",
    )
    .unwrap();
}
#[when("the directory is run without fail-fast")]
async fn run_directory(world: &mut Report) {
    let mut output = Vec::new();
    Execution::cucumber()
        .steps(Collection::new().given(
            None,
            Regex::new("^accepted$").unwrap(),
            pass,
        ))
        .with_messages(&mut output)
        .with_default_cli()
        .run(world.directory.as_ref().unwrap().path())
        .await;
    world.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
}
fn messages<'a>(world: &'a Report, kind: &str) -> Vec<&'a Value> {
    world.messages.iter().filter_map(|m| m.get(kind)).collect()
}
#[then(
    "the report contains the exact captured source and native diagnostic \
        location"
)]
fn exact(world: &mut Report) {
    let sources = messages(world, "source");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["data"], world.source);
    assert_eq!(sources[0]["uri"], world.path.to_str().unwrap());
    let errors = messages(world, "parseError");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["source"]["uri"], world.path.to_str().unwrap());
    let (line, col) = world.location.unwrap();
    assert_eq!(errors[0]["source"]["location"]["line"], line);
    assert_eq!(errors[0]["source"]["location"]["column"], col);
    let message = errors[0]["message"].as_str().unwrap();
    assert!(!message.is_empty());
    if world.kind == "expansion" {
        assert!(message.contains("missing"));
    } else {
        assert!(message.contains("Error at"));
    }
    let source_index =
        world.messages.iter().position(|m| m.get("source").is_some()).unwrap();
    let error_index = world
        .messages
        .iter()
        .position(|m| m.get("parseError").is_some())
        .unwrap();
    assert!(source_index < error_index);
}
#[then("every error-report envelope satisfies the pinned Messages schema")]
fn schema(world: &mut Report) {
    static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        jsonschema::validator_for(
            &serde_json::from_str(include_str!(
                "fixtures/messages/messages-34.2.1.schema.json"
            ))
            .unwrap(),
        )
        .unwrap()
    });
    assert!(world.messages[0].get("meta").is_some());
    assert_eq!(messages(world, "testRunStarted").len(), 1);
    for message in &world.messages {
        let errors = SCHEMA
            .iter_errors(message)
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
}
#[then("the overall run is unsuccessful")]
fn unsuccessful(world: &mut Report) {
    let ends = messages(world, "testRunFinished");
    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0]["success"], false);
    assert_eq!(
        ends[0]["testRunStartedId"],
        messages(world, "testRunStarted")[0]["id"]
    );
}
#[then("the run completes unsuccessfully without invented test cases")]
fn no_cases(world: &mut Report) {
    unsuccessful(world);
    assert!(messages(world, "testCase").is_empty());
    assert!(messages(world, "testCaseStarted").is_empty());
    assert!(messages(world, "testStepFinished").is_empty());
}
#[then(
    "the report identifies the I/O failure without a fabricated parse source"
)]
fn io_failure(world: &mut Report) {
    assert!(messages(world, "source").is_empty());
    assert!(messages(world, "parseError").is_empty());
    let end = messages(world, "testRunFinished")[0];
    assert!(
        end["exception"]["message"]
            .as_str()
            .unwrap()
            .contains("broken.feature")
    );
    assert_eq!(end["exception"]["type"], "InputReadError");
}
#[then("the passing scenario and rejected source are both represented")]
fn both(world: &mut Report) {
    assert_eq!(messages(world, "parseError").len(), 1);
    assert_eq!(messages(world, "source").len(), 2);
    assert_eq!(messages(world, "testCase").len(), 1);
    assert_eq!(
        messages(world, "testStepFinished")[0]["testStepResult"]["status"],
        "PASSED"
    );
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_errors")
        .await;
}
