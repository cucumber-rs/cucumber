//! Phase 12 BDD: actual official JavaScript Query and JUnit consumers.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    collections::HashMap,
    io::Write as _,
    process::Stdio,
    sync::{LazyLock, Mutex},
};

use cucumber::{
    World as _, given,
    step::{Collection, Context, Matching},
    then, when,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;
#[derive(Debug, Default, cucumber::World)]
struct Report {
    output: Vec<u8>,
    consumed: Value,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution;
static COUNTS: LazyLock<Mutex<HashMap<String, usize>>> =
    LazyLock::new(Default::default);
fn act(_: &mut Execution, ctx: Context) -> LocalBoxFuture<'_, ()> {
    let text = ctx.step.value.clone();
    if let Some(amount) = text.strip_prefix("amount ") {
        assert_eq!(
            ctx.step.docstring.as_deref(),
            Some(format!("paid {amount}").as_str())
        );
        assert_eq!(ctx.step.table.as_ref().unwrap().rows[1][0], amount);
    }
    let count = {
        let mut counts = COUNTS.lock().unwrap();
        let count = counts.entry(text.clone()).or_default();
        *count += 1;
        *count
    };
    Box::pin(async move {
        if text == "exhausted" || (text == "recovered" && count == 1) {
            panic!("deliberate downstream failure");
        }
    })
}
async fn produce(report: &mut Report, source: &str, retries: usize) {
    COUNTS.lock().unwrap().clear();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("consumer.feature");
    std::fs::write(&path, source).unwrap();
    let steps = Collection::new().given(None, Regex::new(".*").unwrap(), act);
    Execution::cucumber()
        .steps(steps)
        .step_matching(Matching::KeywordIndependent)
        .with_dialect_catalog(cucumber::parser::DialectCatalog::Gherkin43)
        .unwrap()
        .with_docstring_semantics(
            cucumber::gherkin::DocStringSemantics::Gherkin43,
        )
        .retries(retries)
        .max_concurrent_scenarios(2)
        .before(|_, _, _, _| Box::pin(async {}))
        .after(|_, _, _, _, _| Box::pin(async {}))
        .with_messages(&mut report.output)
        .with_default_cli()
        .run(&path)
        .await;
}
#[given("a rich native execution report")]
async fn rich(report: &mut Report) {
    produce(
        report,
        "\
            @invoice\n\
            Feature: Billing\n\
            \x20 @policy\n\
            \x20 Rule: Tax\n\
            \x20   Background: account\n\
            \x20     Given ready\n\
            \x20   Scenario Outline: invoice <amount>\n\
            \x20     Given amount <amount>\n\
            \x20       | amount |\n\
            \x20       | <amount> |\n\
            \x20       \"\"\"text/plain\n\
            \x20       paid <amount>\n\
            \x20       \"\"\"\n\
            \x20     Examples: charges\n\
            \x20       | amount |\n\
            \x20       | 42 |\n\
            \x20       | 43 |\n",
        0,
    )
    .await;
}
#[given("a mixed retry execution report")]
async fn retry(report: &mut Report) {
    produce(
        report,
        "Feature: Attempts\n  Scenario: recovered\n    Given recovered\n  \
            Scenario: exhausted\n    Given exhausted\n",
        1,
    )
    .await;
}
#[when("the pinned official JavaScript consumers read the report")]
fn consume(report: &mut Report) {
    let mut child = oracle::command("tests/interop/consume.mjs")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node required for official-consumer BDD");
    child.stdin.take().unwrap().write_all(&report.output).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "official consumer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    report.consumed = serde_json::from_slice(&output.stdout).unwrap();
}
#[then("the official query resolves both example rows and their arguments")]
fn rich_query(report: &mut Report) {
    let rows = report.consumed["outcomes"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for (amount, line) in [("42", 16), ("43", 17)] {
        let row = rows
            .iter()
            .find(|r| r["name"] == format!("invoice {amount}"))
            .unwrap();
        assert_eq!(row["feature"], "Billing");
        assert_eq!(row["rule"], "Tax");
        assert_eq!(row["exampleLine"], line);
        assert_eq!(row["tags"], serde_json::json!(["@invoice", "@policy"]));
        assert_eq!(row["status"], "PASSED");
        let steps = row["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0]["hookType"], "BEFORE_TEST_CASE");
        assert_eq!(steps[3]["hookType"], "AFTER_TEST_CASE");
        let step = &steps[2];
        assert_eq!(step["text"], format!("amount {amount}"));
        assert_eq!(step["sourceText"], "amount <amount>");
        assert_eq!(
            step["argument"]["docString"]["content"],
            format!("paid {amount}")
        );
        assert_eq!(step["argument"]["docString"]["mediaType"], "text/plain");
        assert_eq!(
            step["argument"]["dataTable"]["rows"][1]["cells"][0]["value"],
            amount
        );
    }
}
fn assert_xml(report: &Report, failures: usize) {
    let xml = report.consumed["xml"].as_str().unwrap();
    let suite = Regex::new(r"(?s)<testsuite\b[^>]*>")
        .unwrap()
        .find(xml)
        .unwrap()
        .as_str();
    for (key, value) in
        [("tests", 2), ("failures", failures), ("errors", 0), ("skipped", 0)]
    {
        assert!(suite.contains(&format!("{key}=\"{value}\"")), "{xml}");
    }
    assert_eq!(xml.matches("<testcase ").count(), 2, "{xml}");
    assert_eq!(xml.matches("<failure").count(), failures, "{xml}");
}
#[then("the official JUnit formatter reports two passing examples")]
fn passing(report: &mut Report) {
    assert_xml(report, 0);
    let xml = report.consumed["xml"].as_str().unwrap();
    assert!(xml.contains("invoice 42"));
    assert!(xml.contains("invoice 43"));
    assert!(xml.contains("classname=\"Billing\""));
}
#[then("the official query retains two final outcomes from four attempts")]
fn retry_query(report: &mut Report) {
    assert_eq!(report.consumed["observedAttempts"], 4);
    let rows = report.consumed["outcomes"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for (name, status) in [("recovered", "PASSED"), ("exhausted", "FAILED")] {
        let row = rows.iter().find(|r| r["name"] == name).unwrap();
        assert_eq!(row["attempt"], 1);
        assert_eq!(row["status"], status);
        let steps = row["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0]["hookType"], "BEFORE_TEST_CASE");
        assert_eq!(steps[2]["hookType"], "AFTER_TEST_CASE");
    }
    assert_eq!(report.consumed["counts"]["PASSED"], 1);
    assert_eq!(report.consumed["counts"]["FAILED"], 1);
}
#[then("the official JUnit formatter reports one passing and one failing case")]
fn failed(report: &mut Report) {
    assert_xml(report, 1);
    assert!(
        report.consumed["xml"]
            .as_str()
            .unwrap()
            .contains("deliberate downstream failure")
    );
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_consumer")
        .await;
}
