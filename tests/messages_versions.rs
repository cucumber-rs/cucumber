//! Phase22 explicit protocol schema acceptance.
use std::sync::{LazyLock, Mutex};

use cucumber::{
    World as _, given,
    step::{Collection, Context},
    then, when,
    writer::messages::ProtocolVersion,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;
#[derive(Debug, Default, cucumber::World)]
struct Report {
    version: Option<ProtocolVersion>,
    messages: Vec<Value>,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution;
static ATTEMPTS: LazyLock<Mutex<usize>> = LazyLock::new(Default::default);
fn execute(_: &mut Execution, ctx: Context) -> LocalBoxFuture<'_, ()> {
    let mut attempts = ATTEMPTS.lock().unwrap();
    *attempts += 1;
    let attempt = *attempts;
    drop(attempts);
    assert_eq!(ctx.step.docstring.as_deref(), Some("paid 42"));
    assert_eq!(ctx.step.table.as_ref().unwrap().rows[1][0], "42");
    Box::pin(async move {
        assert!(attempt > 1, "first attempt only");
    })
}
#[given(expr = "a source-backed reporter configured for {string}")]
fn select(world: &mut Report, version: String) {
    world.version = Some(match version.as_str() {
        "34.2.1" => ProtocolVersion::V34,
        "35.0.0" => ProtocolVersion::V35,
        _ => panic!("unknown"),
    });
}
#[given("a source-backed reporter using its default protocol")]
fn default_protocol(world: &mut Report) {
    world.version = None;
}
#[when("a rich outline with native hooks and retries executes")]
async fn run(world: &mut Report) {
    *ATTEMPTS.lock().unwrap() = 0;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("version.feature");
    std::fs::write(
        &path,
        "\
            @invoice\n\
            Feature: Protocol\n\
            \x20 Rule: Shared\n\
            \x20   Scenario Outline: invoice <amount>\n\
            \x20     Given payment <amount>\n\
            \x20       | amount |\n\
            \x20       | <amount> |\n\
            \x20       \"\"\"text/plain\n\
            \x20       paid <amount>\n\
            \x20       \"\"\"\n\
            \x20     Examples:\n\
            \x20       | amount |\n\
            \x20       | 42 |\n",
    )
    .unwrap();
    let mut output = Vec::new();
    let runner = Execution::cucumber()
        .steps(Collection::new().given(
            None,
            Regex::new("^payment 42$").unwrap(),
            execute,
        ))
        .with_docstring_semantics(
            cucumber::gherkin::DocStringSemantics::Gherkin43,
        )
        .retries(1)
        .before(|_, _, _, _| Box::pin(async {}))
        .after(|_, _, _, _, _| Box::pin(async {}))
        .with_default_cli();
    if let Some(protocol) = world.version {
        runner
            .with_messages_protocol(&mut output, protocol)
            .with_default_cli()
            .run(&path)
            .await;
    } else {
        runner.with_messages(&mut output).with_default_cli().run(&path).await;
    }
    world.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
}
#[then(expr = "the report declares {string} and satisfies its official schema")]
fn schema(world: &mut Report, version: String) {
    assert_eq!(world.messages[0]["meta"]["protocolVersion"], version);
    let schema = if version == "34.2.1" {
        include_str!("fixtures/messages/messages-34.2.1.schema.json")
    } else {
        include_str!("fixtures/messages/messages-35.0.0.schema.json")
    };
    let validator =
        jsonschema::validator_for(&serde_json::from_str(schema).unwrap())
            .unwrap();
    for message in &world.messages {
        let errors = validator
            .iter_errors(message)
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
}
#[then("source, argument, hook and final attempt outcomes remain intact")]
fn outcomes(world: &mut Report) {
    let messages = |kind: &str| {
        world
            .messages
            .iter()
            .filter_map(move |m| m.get(kind))
            .collect::<Vec<_>>()
    };
    assert_eq!(messages("source").len(), 1);
    assert_eq!(messages("gherkinDocument").len(), 1);
    assert_eq!(messages("pickle").len(), 1);
    let pickle = messages("pickle")[0];
    assert_eq!(pickle["name"], "invoice 42");
    assert_eq!(
        pickle["steps"][0]["argument"]["docString"]["content"],
        "paid 42"
    );
    let table = &pickle["steps"][0]["argument"]["dataTable"];
    assert_eq!(table["rows"][1]["cells"][0]["value"], "42");
    assert_eq!(messages("hook").len(), 2);
    assert_eq!(messages("testCase").len(), 1);
    let starts = messages("testCaseStarted");
    assert_eq!(starts.len(), 2);
    assert_eq!(starts[0]["attempt"], 0);
    assert_eq!(starts[1]["attempt"], 1);
    let ends = messages("testCaseFinished");
    assert_eq!(ends[0]["willBeRetried"], true);
    assert_eq!(ends[1]["willBeRetried"], false);
    assert_eq!(messages("testRunFinished")[0]["success"], true);
    assert_eq!(
        messages("testStepFinished")
            .iter()
            .filter(|s| s["testStepResult"]["status"] == "FAILED")
            .count(),
        1
    );
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_versions")
        .await;
}
