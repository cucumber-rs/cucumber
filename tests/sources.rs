//! Phase 6a behavior acceptance for the parser/report source boundary.
use std::{path::PathBuf, sync::Arc};

use cucumber::{
    Parser as _, StatsWriter as _, World as _, WriterExt as _, given,
    parser::{Basic, DialectCatalog, SourceSnapshot, SourceStore, basic::Cli},
    then, when,
};
use futures::StreamExt as _;

#[derive(Debug, Default, cucumber::World)]
struct Sources {
    directory: Option<tempfile::TempDir>,
    path: PathBuf,
    original: String,
    store: SourceStore,
    retained: Option<Arc<SourceSnapshot>>,
    parsed: Vec<cucumber::gherkin::Feature>,
    configured: Option<Basic>,
    policy_error: Option<String>,
    runtime_payloads: Vec<String>,
    runtime_stats: (usize, usize, usize, usize, usize),
    runtime_output: String,
    table_error: Option<cucumber::parser::Error>,
}

fn write_source(world: &mut Sources, source: &str) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.feature");
    std::fs::write(&path, source).unwrap();
    world.path = path.canonicalize().unwrap();
    world.original = source.to_owned();
    world.directory = Some(directory);
}

#[given("a source document without a final newline")]
fn plain(world: &mut Sources) {
    write_source(
        world,
        "Feature: Original\n  Scenario: Source\n    Given exact input",
    );
}

#[given("a source document containing an outline")]
fn outline(world: &mut Sources) {
    write_source(
        world,
        "Feature: Payments\n  Scenario Outline: invoice <amount>\n    Given \
            amount <amount>\n    Examples:\n      | amount |\n      | 42 \
            |\n      | 43 |\n",
    );
}

#[given("a French source document without a language directive")]
fn french(world: &mut Sources) {
    write_source(
        world,
        "Fonctionnalité: Source\n  Scénario: Original\n    Soit exact input\n",
    );
}

async fn capture(world: &mut Sources, language: &str) {
    let result = Basic::new()
        .with_source_store(world.store.clone())
        .language(language.to_owned())
        .unwrap()
        .parse(&world.path, Cli::default())
        .collect::<Vec<_>>()
        .await;
    world.parsed = result.into_iter().map(Result::unwrap).collect();
    assert_eq!(world.parsed.len(), 1);
    world.retained = world.store.get(&world.path);
    assert!(world.retained.is_some());
}

#[when(regex = "^the parser captures the source and the file is \
        (rewritten|deleted)$")]
async fn mutate(world: &mut Sources, change: String) {
    capture(world, "en").await;
    if change == "rewritten" {
        std::fs::write(&world.path, "Feature: Different source\n").unwrap();
    } else {
        std::fs::remove_file(&world.path).unwrap();
    }
}

#[when("the parser captures the source and expands its examples")]
async fn expand(world: &mut Sources) {
    capture(world, "en").await;
}

#[when("the French parser captures the source")]
async fn capture_french(world: &mut Sources) {
    capture(world, "fr").await;
}

#[when("the same path is parsed again with different content")]
async fn again(world: &mut Sources) {
    capture(world, "en").await;
    std::fs::write(
        &world.path,
        "Feature: Replacement\n  Scenario: Later\n    Given new input\n",
    )
    .unwrap();
    let results = Basic::new()
        .with_source_store(world.store.clone())
        .parse(&world.path, Cli::default())
        .collect::<Vec<_>>()
        .await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].as_ref().unwrap().name, "Replacement");
}

#[then("the snapshot contains the exact original bytes")]
#[then("the retained first snapshot contains the original bytes")]
fn exact(world: &mut Sources) {
    let snapshot = world.retained.as_ref().unwrap();
    assert_eq!(snapshot.source().as_bytes(), world.original.as_bytes());
    assert!(!snapshot.source().ends_with('\n'));
}

#[then("the captured native feature matches the parsed feature")]
fn ast(world: &mut Sources) {
    assert_eq!(world.retained.as_ref().unwrap().feature(), &world.parsed[0]);
}

#[then("the snapshot still contains the outline template")]
fn template(world: &mut Sources) {
    let snapshot = world.retained.as_ref().unwrap();
    assert_eq!(snapshot.source(), world.original);
    assert_eq!(snapshot.feature().scenarios.len(), 1);
    let scenario = &snapshot.feature().scenarios[0];
    assert_eq!(scenario.name, "invoice <amount>");
    assert_eq!(scenario.steps[0].value, "amount <amount>");
    assert_eq!(scenario.examples[0].table.as_ref().unwrap().rows.len(), 3);
}

#[then("the runner input contains both expanded examples")]
fn expanded(world: &mut Sources) {
    let scenarios = &world.parsed[0].scenarios;
    assert_eq!(scenarios.len(), 2);
    assert_eq!(scenarios[0].name, "invoice 42");
    assert_eq!(scenarios[0].steps[0].value, "amount 42");
    assert_eq!(scenarios[1].name, "invoice 43");
    assert_eq!(scenarios[1].steps[0].value, "amount 43");
}

#[then("the snapshot records the French default dialect")]
fn dialect(world: &mut Sources) {
    assert_eq!(world.retained.as_ref().unwrap().default_language(), "fr");
    assert_eq!(world.parsed[0].keyword, "Fonctionnalité");
}

#[then("the current snapshot contains the newly parsed bytes")]
fn generation(world: &mut Sources) {
    let latest = world.store.get(&world.path).unwrap();
    assert_eq!(
        latest.source(),
        "Feature: Replacement\n  Scenario: Later\n    Given new input\n"
    );
    assert_eq!(latest.feature().name, "Replacement");
    assert!(!Arc::ptr_eq(world.retained.as_ref().unwrap(), &latest));
}

#[given("a Texan source using the corrected Rule keyword")]
fn texan(world: &mut Sources) {
    write_source(
        world,
        "# language: en-tx\nThis ain’t my first rodeo: Source\n  Rule: \
            Published vocabulary\n    All hat and no cattle: Input\n      \
            Fixin' to retain source\n",
    );
}

#[when("the parser captures it with the Gherkin 43 catalogue")]
async fn latest(world: &mut Sources) {
    world.parsed = Basic::new()
        .with_source_store(world.store.clone())
        .with_dialect_catalog(DialectCatalog::Gherkin43)
        .unwrap()
        .parse(&world.path, Cli::default())
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(Result::unwrap)
        .collect();
    world.retained = world.store.get(&world.path);
}

#[then("the captured feature contains the Texan rule and its scenario")]
fn rule(world: &mut Sources) {
    let feature = world.retained.as_ref().unwrap().feature();
    assert_eq!(feature.rules.len(), 1);
    assert_eq!(feature.rules[0].name, "Published vocabulary");
    assert_eq!(feature.rules[0].scenarios.len(), 1);
    assert_eq!(feature.rules[0].scenarios[0].steps[0].value, "retain source");
    assert_eq!(feature, &world.parsed[0]);
}

#[then("the snapshot records the Gherkin 43 catalogue")]
fn latest_recorded(world: &mut Sources) {
    assert_eq!(
        world.retained.as_ref().unwrap().dialect_catalog(),
        DialectCatalog::Gherkin43
    );
}

#[then("the snapshot records the legacy catalogue")]
fn legacy_recorded(world: &mut Sources) {
    assert_eq!(
        world.retained.as_ref().unwrap().dialect_catalog(),
        DialectCatalog::Legacy
    );
}

#[given("a parser configured for the legacy formal dialect")]
fn formal(world: &mut Sources) {
    world.configured = Some(Basic::new().language("formal").unwrap());
}

#[when("the parser selects the Gherkin 43 catalogue")]
fn switch(world: &mut Sources) {
    let result = world
        .configured
        .take()
        .unwrap()
        .with_dialect_catalog(DialectCatalog::Gherkin43);
    world.policy_error = result.err().map(|error| error.0.to_string());
}

#[then("the catalogue change reports the unsupported formal dialect")]
fn invalid_policy(world: &mut Sources) {
    assert_eq!(world.policy_error.as_deref(), Some("formal"));
}

#[derive(Debug, Default, cucumber::World)]
struct PayloadApplication {
    payloads: Vec<String>,
}

#[given("record the payload")]
fn record_payload(
    world: &mut PayloadApplication,
    step: &cucumber::gherkin::Step,
) {
    world
        .payloads
        .push(step.docstring.clone().expect("doc string reaches handler"));
}

#[given("an application feature with a typed doc-string payload")]
fn typed_payload(world: &mut Sources) {
    write_source(
        world,
        "Feature: Payload delivery\n  Scenario: Typed payload\n    Given \
            record the payload\n      \"\"\"application/json\n      \
            {\"amount\": 42}\n        approved\n      \"\"\"\n",
    );
}

#[given("an application outline with two doc-string payload examples")]
fn outline_payload(world: &mut Sources) {
    write_source(
        world,
        "\
            Feature: Payload delivery\n\
            \x20 Scenario Outline: invoice <amount>\n\
            \x20   Given record the payload\n\
            \x20     \"\"\"application/json\n\
            \x20     {\"amount\": <amount>}\n\
            \x20       invoice <amount>\n\
            \x20     \"\"\"\n\
            \x20   Examples:\n\
            \x20     | amount |\n\
            \x20     | 42 |\n\
            \x20     | 43 |\n",
    );
}

#[given("an application feature with a legacy doc-string payload")]
fn legacy_payload(world: &mut Sources) {
    write_source(
        world,
        "Feature: Payload delivery\n  Scenario: Legacy payload\n    Given \
            record the payload\n      \"\"\"\n      legacy payload\n      \
            \"\"\"\n",
    );
}

async fn run_payload_application(
    world: &mut Sources,
    semantics: Option<cucumber::gherkin::DocStringSemantics>,
) {
    let payloads = Arc::new(std::sync::Mutex::new(Vec::new()));
    let finished = Arc::clone(&payloads);
    let mut output = Vec::new();
    let application = PayloadApplication::cucumber()
        .with_source_store(world.store.clone())
        .max_concurrent_scenarios(1)
        .after(move |_, _, _, _, application| {
            let finished = Arc::clone(&finished);
            Box::pin(async move {
                if let Some(application) = application {
                    finished
                        .lock()
                        .unwrap()
                        .extend(application.payloads.clone());
                }
            })
        })
        .with_writer(
            cucumber::writer::Basic::new(
                &mut output,
                cucumber::writer::Coloring::Never,
                0,
            )
            .summarized(),
        )
        .with_default_cli();
    let application = if let Some(semantics) = semantics {
        application.with_docstring_semantics(semantics)
    } else {
        application
    };
    let report = application.run(&world.path).await;
    world.runtime_stats = (
        report.passed_steps(),
        report.failed_steps(),
        report.skipped_steps(),
        report.hook_errors(),
        report.parsing_errors(),
    );
    drop(report);
    world.runtime_output = String::from_utf8(output).unwrap();
    world.runtime_payloads = payloads.lock().unwrap().clone();
    world.retained = world.store.get(&world.path);
}

#[when("the application runs with official doc-string semantics")]
async fn run_official_payloads(world: &mut Sources) {
    run_payload_application(
        world,
        Some(cucumber::gherkin::DocStringSemantics::Gherkin43),
    )
    .await;
}

#[when("the application runs with its default parser")]
async fn run_default_payloads(world: &mut Sources) {
    run_payload_application(world, None).await;
}

fn assert_payloads(world: &Sources, expected: Vec<String>) {
    assert_eq!(
        world.runtime_stats,
        (expected.len(), 0, 0, 0, 0),
        "{}",
        world.runtime_output
    );
    assert_eq!(world.runtime_payloads, expected);
}

#[then("the handler receives the normalized payload exactly")]
fn normalized_payload(world: &mut Sources) {
    assert_payloads(world, vec!["{\"amount\": 42}\n  approved".to_owned()]);
}

#[then("both handlers receive their expanded normalized payloads exactly")]
fn normalized_outline_payloads(world: &mut Sources) {
    assert_payloads(
        world,
        vec![
            "{\"amount\": 42}\n  invoice 42".to_owned(),
            "{\"amount\": 43}\n  invoice 43".to_owned(),
        ],
    );
}

#[then("the handler receives the legacy payload including its edge newlines")]
fn unchanged_legacy_payload(world: &mut Sources) {
    assert_payloads(world, vec!["\nlegacy payload\n".to_owned()]);
}

#[then("the snapshot keeps the unexpanded normalized doc-string template")]
fn snapshot_docstring_template(world: &mut Sources) {
    let feature = world.retained.as_ref().unwrap().feature();
    assert_eq!(feature.scenarios.len(), 1);
    assert_eq!(feature.scenarios[0].name, "invoice <amount>");
    assert_eq!(
        feature.scenarios[0].steps[0].docstring.as_deref(),
        Some("{\"amount\": <amount>}\n  invoice <amount>")
    );
}

#[then(
    "the snapshot records official doc-string semantics and the original \
        source"
)]
fn official_content_snapshot(world: &mut Sources) {
    let snapshot = world.retained.as_ref().unwrap();
    assert_eq!(
        snapshot.docstring_semantics(),
        cucumber::gherkin::DocStringSemantics::Gherkin43
    );
    assert_eq!(snapshot.source(), world.original);
    assert_eq!(
        snapshot.dialect_catalog(),
        DialectCatalog::Legacy,
        "content policy must not silently change vocabulary"
    );
}

#[then(
    "the snapshot records legacy doc-string semantics and the original source"
)]
fn legacy_content_snapshot(world: &mut Sources) {
    let snapshot = world.retained.as_ref().unwrap();
    assert_eq!(
        snapshot.docstring_semantics(),
        cucumber::gherkin::DocStringSemantics::Legacy
    );
    assert_eq!(snapshot.source(), world.original);
    assert_eq!(
        snapshot.feature().scenarios[0].steps[0].docstring.as_deref(),
        Some("\nlegacy payload\n")
    );
}

#[tokio::main]
async fn main() {
    Sources::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/sources")
        .await;
}

#[given(expr = "an application {string} table with a {string} later row")]
fn application_table(world: &mut Sources, kind: String, width: String) {
    let row = match width.as_str() {
        "shorter" => "| first |",
        "longer" => "| first | second | discarded |",
        "matching" => "| first | second |",
        _ => panic!("Unknown table width: {width}"),
    };
    let prefix = match kind.as_str() {
        "step" => {
            "Feature: Table delivery\n  Scenario: Payload\n    Given record \
                the table\n"
        }
        "Examples" => {
            "Feature: Table delivery\n  Scenario Outline: Payload\n    Given \
                record table values <name> <value>\n    Examples:\n"
        }
        _ => panic!("Unknown table kind: {kind}"),
    };
    write_source(
        world,
        &format!("{prefix}      | name | value |\n      {row}\n"),
    );
}

#[given("record the table")]
fn record_table(
    world: &mut PayloadApplication,
    step: &cucumber::gherkin::Step,
) {
    let table = step.table.as_ref().expect("table reaches handler");
    assert_eq!(
        table.rows,
        vec![vec!["name", "value"], vec!["first", "second"]]
    );
    world.payloads.push(format!("{}:{}", table.rows[1][0], table.rows[1][1]));
}

#[given(expr = "record table values {word} {word}")]
fn record_table_values(
    world: &mut PayloadApplication,
    first: String,
    second: String,
) {
    world.payloads.push(format!("{first}:{second}"));
}

#[when(expr = "the table application runs with {string} sizing")]
async fn run_table_application(world: &mut Sources, policy: String) {
    let payloads = Arc::new(std::sync::Mutex::new(Vec::new()));
    let finished = Arc::clone(&payloads);
    let mut output = Vec::new();
    let application = PayloadApplication::cucumber()
        .with_source_store(world.store.clone())
        .max_concurrent_scenarios(1)
        .after(move |_, _, _, _, application| {
            let finished = Arc::clone(&finished);
            Box::pin(async move {
                if let Some(application) = application {
                    finished
                        .lock()
                        .unwrap()
                        .extend(application.payloads.clone());
                }
            })
        })
        .with_writer(
            cucumber::writer::Basic::new(
                &mut output,
                cucumber::writer::Coloring::Never,
                0,
            )
            .summarized(),
        )
        .with_default_cli();
    let application = match policy.as_str() {
        "Gherkin 43" => application
            .with_table_semantics(cucumber::gherkin::TableSemantics::Gherkin43),
        "default" => application,
        _ => panic!("Unknown table policy: {policy}"),
    };
    let report = application.run(&world.path).await;
    world.runtime_stats = (
        report.passed_steps(),
        report.failed_steps(),
        report.skipped_steps(),
        report.hook_errors(),
        report.parsing_errors(),
    );
    drop(report);
    world.runtime_output = String::from_utf8(output).unwrap();
    world.runtime_payloads = payloads.lock().unwrap().clone();
    world.retained = world.store.get(&world.path);
}

#[then("one parse error prevents every table handler from running")]
fn rejected_table_application(world: &mut Sources) {
    assert_eq!(
        world.runtime_stats,
        (0, 0, 0, 0, 1),
        "{}",
        world.runtime_output
    );
    assert!(world.runtime_payloads.is_empty());
}

#[then("the invalid table source has no successful snapshot")]
fn no_invalid_table_snapshot(world: &mut Sources) {
    assert!(world.retained.is_none());
    assert!(world.store.get(&world.path).is_none());
}

#[then(expr = "the {string} table values reach the handler exactly")]
fn table_handler_values(world: &mut Sources, kind: String) {
    let expected = match kind.as_str() {
        "step" => "first:second",
        "Examples" => "first:second",
        _ => panic!("Unknown table kind: {kind}"),
    };
    assert_payloads(world, vec![expected.to_owned()]);
}

#[then(expr = "the source snapshot records {string} table sizing")]
fn table_snapshot_policy(world: &mut Sources, policy: String) {
    let snapshot = world.retained.as_ref().expect("successful parse captured");
    let expected = match policy.as_str() {
        "Gherkin 43" => cucumber::gherkin::TableSemantics::Gherkin43,
        "Legacy" => cucumber::gherkin::TableSemantics::Legacy,
        _ => panic!("Unknown table policy: {policy}"),
    };
    assert_eq!(snapshot.table_semantics(), expected);
    assert_eq!(snapshot.source(), world.original);
    assert_eq!(snapshot.dialect_catalog(), DialectCatalog::Legacy);
    assert_eq!(
        snapshot.docstring_semantics(),
        cucumber::gherkin::DocStringSemantics::Legacy
    );
    assert_eq!(
        snapshot.example_semantics(),
        cucumber::feature::ExampleSemantics::Legacy
    );
}

#[when("Basic parses the table with strict sizing")]
async fn basic_strict_table(world: &mut Sources) {
    let mut results = Basic::new()
        .with_table_semantics(cucumber::gherkin::TableSemantics::Gherkin43)
        .with_source_store(world.store.clone())
        .parse(&world.path, Cli::default())
        .collect::<Vec<_>>()
        .await;
    assert_eq!(results.len(), 1);
    world.table_error =
        Some(results.pop().unwrap().expect_err("ragged table rejected"));
}

#[then("Basic reports the inconsistent-cell-count diagnostic")]
fn basic_table_diagnostic(world: &mut Sources) {
    match world.table_error.as_ref().expect("Basic invoked") {
        cucumber::parser::Error::Parsing(error) => match error.as_ref() {
            cucumber::gherkin::ParseFileError::Parsing {
                error: Some(EnvError::InconsistentCellCount(rows)),
                ..
            } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(
                    rows[1].len(),
                    3,
                    "original invalid cells retained for diagnosis"
                );
            }
            other => panic!("Expected table diagnostic, got {other:?}"),
        },
        other => panic!("Expected parsing failure, got {other:?}"),
    }
    assert!(world.store.get(&world.path).is_none());
}

use cucumber::gherkin::EnvError;
