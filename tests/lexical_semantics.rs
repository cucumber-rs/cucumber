//! Phase38: whole-graph differential BDD, native execution and original bytes.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    collections::{HashMap, HashSet},
    io::Write as _,
    path::PathBuf,
    process::Stdio,
    sync::{LazyLock, Mutex},
};

use cucumber::{
    Parser as _, World as _,
    filter::LineSelectors,
    gherkin, given,
    parser::{self, SourceStore},
    step::{Collection, Context},
    then, when,
    writer::{
        Messages,
        messages::{ProtocolVersion, source::SourceGraph},
    },
};
use futures::{StreamExt as _, future::LocalBoxFuture};
use regex::Regex;
use serde_json::{Value, json};
static OBSERVED: Mutex<Vec<Value>> = Mutex::new(Vec::new());
#[derive(Debug, Default, cucumber::World)]
struct Lexical {
    source: String,
    path: PathBuf,
    directory: Option<tempfile::TempDir>,
    store: SourceStore,
    official: Option<Value>,
    messages: Vec<Value>,
    observed: Vec<Value>,
    legacy: Option<gherkin::Feature>,
    legacy_error: bool,
    official_error: bool,
    generations_ok: bool,
    legacy_catalogue: bool,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution {
    name: String,
    tags: Vec<String>,
}
fn record(world: &mut Execution, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        OBSERVED.lock().unwrap().push(json!({
            "name":world.name,
            "tags":world.tags,
            "text":context.step.value,

        "doc":context.step.docstring,
            "table":context.step.table.map(|table|table.rows)}
            ));
    })
}
fn padding(name: &str) -> &'static str {
    match name {
        "NBSP" => "\u{a0}",
        "BOM" => "\u{feff}",
        "EM SPACE" => "\u{2003}",
        "IDEOGRAPHIC SPACE" => "\u{3000}",
        "vertical tab" => "\u{b}",
        "form feed" => "\u{c}",
        "carriage return" => "\r",
        "line separator" => "\u{2028}",
        "paragraph separator" => "\u{2029}",
        _ => panic!("unknown padding {name}"),
    }
}
#[given(
    expr = "a classic source padded with {string} around declarations tags \
        steps and Examples"
)]
fn padded(world: &mut Lexical, name: String) {
    let p = padding(&name);
    let lines = vec![
        format!("# language: en{p}"),
        "# retained comment".into(),
        format!("@feature{p}"),
        format!("Feature:{p}Lexical{p}"),
        "".into(),
        format!("Background:{p}prepare{p}"),
        format!("Given {p}prepare{p}"),
        "@rule".into(),
        format!("Rule:{p}invoices{p}"),
        format!("Background:{p}scope{p}"),
        format!("Given {p}scope{p}"),
        "@scenario".into(),
        format!("Scenario Outline:{p}invoice <value>{p}"),
        format!("Given {p}value <value>{p}"),
        format!("\"\"\"{p}text/plain{p}"),
        "payload <value>".into(),
        "\"\"\"".into(),
        "| cell |".into(),
        "| <value> |".into(),
        "@example".into(),
        format!("Examples:{p}rows{p}"),
        "| value |".into(),
        "| green |".into(),
        "| blue |".into(),
    ];
    world.source =
        lines.into_iter().map(|line| format!("{p}{line}\n")).collect();
}
#[given(expr = "the lexical source case {string}")]
fn source_case(world: &mut Lexical, case: String) {
    world.source = match case.as_str() {
        "BOM trims titles and step text" => "Feature: \u{feff}F\u{feff}\n \
        Scenario: \u{feff}N\u{feff}\n  Given \u{feff}value\u{feff}\n"
            .into(),

        "NEL remains title and step text" => "Feature: F\u{85}\n Scenario: \
        N\u{85}\n  Given value\u{85}\n"
            .into(),

        "NEL remains inside a tag" => "@a\u{85}b\nFeature: F\n Scenario: N\n  \
        Given value\n"
            .into(),

        "BOM separates tags" => {
            "@a\u{feff}@b\nFeature: F\n Scenario: N\n  Given \
        value\n"
                .into()
        }

        "Unicode-indented directive and comment" => "\u{a0}#\u{feff}language:\
        \u{2003}fr\u{a0}\n\u{2003}# commentaire\n\u{a0}Fonctionnalité: \
        F\n\u{2003}Scénario: N\n\u{feff}Soit value\n"
            .into(),

        "Unicode whitespace before a trailing tag comment" => {
            "@a\u{a0}# trailing \
        comment\nFeature: F\n Scenario: N\n  Given value\n"
                .into()
        }

        "NEL before Feature" => {
            "\u{85}Feature: F\n Scenario: N\n  Given value\n".into()
        }

        "NBSP inside a tag" => {
            "@one\u{a0}two\nFeature: F\n Scenario: N\n  Given \
        value\n"
                .into()
        }

        _ => panic!("unknown source case {case}"),
    };
}
#[given(expr = "a featureless source containing {string}")]
fn featureless(world: &mut Lexical, name: String) {
    world.source = format!("{}\n{}", padding(&name), padding(&name));
}
fn oracle(source: &str) -> Result<Value, String> {
    let mut child = oracle::command("tests/interop/gherkin43/oracle.mjs")
        .args(["official.feature"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pinned Gherkin43 oracle installed");
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout).unwrap())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }
}
fn prepare(world: &mut Lexical) {
    let directory = tempfile::tempdir().unwrap();
    world.path =
        directory.path().canonicalize().unwrap().join("lexical.feature");
    std::fs::write(&world.path, &world.source).unwrap();
    world.directory = Some(directory);
}
async fn run(world: &mut Lexical, lines: Option<LineSelectors>) {
    OBSERVED.lock().unwrap().clear();
    let mut output = Vec::new();
    let app = Execution::cucumber()
        .steps(Collection::new().given(
            None,
            Regex::new("(?s)^.*$").unwrap(),
            record,
        ))
        .max_concurrent_scenarios(1);
    let app = if world.legacy_catalogue {
        app.with_docstring_semantics(gherkin::DocStringSemantics::Gherkin43)
            .with_table_semantics(gherkin::TableSemantics::Gherkin43)
            .with_example_semantics(
                cucumber::feature::ExampleSemantics::Gherkin43,
            )
    } else {
        app.with_gherkin43().unwrap()
    };
    let app = app
        .with_source_store(world.store.clone())
        .with_lexical_semantics(gherkin::LexicalSemantics::Gherkin43)
        .before(|feature, rule, scenario, world| {
            Box::pin(async move {
                world.name = scenario.name.clone();
                world.tags = feature
                    .tags
                    .iter()
                    .chain(rule.into_iter().flat_map(|rule| &rule.tags))
                    .chain(&scenario.tags)
                    .map(|tag| format!("@{tag}"))
                    .collect();
            })
        });
    let app = if let Some(lines) = lines {
        app.with_line_selectors(lines)
    } else {
        app
    };
    app.with_writer(Messages::with_source_store_and_protocol::<Execution>(
        &mut output,
        world.store.clone(),
        ProtocolVersion::V35,
    ))
    .with_default_cli()
    .run(&world.path)
    .await;
    world.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    world.observed = OBSERVED.lock().unwrap().clone();
}
#[when("it is parsed and executed with explicit Gherkin43 lexical semantics")]
async fn execute(world: &mut Lexical) {
    world.official =
        Some(oracle(&world.source).expect("official valid lexical source"));
    prepare(world);
    run(world, None).await;
}
#[when(
    "it runs with Gherkin43 lexical semantics and the Legacy dialect \
        catalogue"
)]
async fn independent(world: &mut Lexical) {
    world.legacy_catalogue = true;
    execute(world).await;
}
#[then(
    "the captured catalogue stays Legacy while lexical semantics are \
        Gherkin43"
)]
fn independent_policies(world: &mut Lexical) {
    let snapshot = world.store.get(&world.path).unwrap();
    assert_eq!(snapshot.dialect_catalog(), parser::DialectCatalog::Legacy);
    assert_eq!(
        snapshot.lexical_semantics(),
        gherkin::LexicalSemantics::Gherkin43
    );
}
#[when("native and official compilers interpret the source")]
async fn interpret(world: &mut Lexical) {
    match oracle(&world.source) {
        Ok(value) => world.official = Some(value),
        Err(_) => world.official_error = true,
    }
    prepare(world);
    run(world, None).await;
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
fn same_graph(expected: &Value, mut actual: Value) {
    let mut ids = HashMap::new();
    collect_ids(expected, &actual, &mut ids, &mut HashSet::new());
    normalize(&mut actual, &ids);
    assert_eq!(
        actual, *expected,
        "all fields, source locations and references must agree"
    );
}
fn reported(world: &Lexical, key: &str) -> Vec<Value> {
    world.messages.iter().filter_map(|m| m.get(key).cloned()).collect()
}
fn schema(world: &Lexical) {
    static VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        jsonschema::validator_for(
            &serde_json::from_str(include_str!(
                "fixtures/messages/messages-35.0.0.schema.json"
            ))
            .unwrap(),
        )
        .unwrap()
    });
    for message in &world.messages {
        let errors = VALIDATOR
            .iter_errors(message)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
}
#[then(
    "the complete AST and Pickles equal the pinned official Gherkin43 \
        compiler"
)]
fn graphs(world: &mut Lexical) {
    let expected = world.official.as_ref().unwrap();
    let documents = reported(world, "gherkinDocument");
    assert_eq!(
        documents.len(),
        1,
        "native envelopes: {:#?}; original source: {:?}",
        world.messages,
        world.source
    );
    same_graph(
        expected,
        json!({
        "gherkinDocument":documents[0],
        "pickles":reported(world,
        "pickle")}
        ),
    );
    let direct =
        SourceGraph::parse_gherkin43("official.feature", &world.source, "en")
            .expect("direct native compiler accepts same original bytes");
    same_graph(
        expected,
        json!({
        "gherkinDocument":direct.gherkin_document(),
        "pickles":direct.pickles()}
        ),
    );
    assert_eq!(reported(world, "testRunFinished")[0]["success"], true);
    schema(world);
}
fn expected_execution(pickles: &[Value]) -> Vec<Value> {
    pickles
        .iter()
        .flat_map(|pickle| {
            pickle["steps"].as_array().unwrap().iter().map(move |step| {
                json!({

                    "name":pickle["name"],
                        "tags":pickle
                            ["tags"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|tag|tag
                            ["name"]
                            .clone())
                            .collect::<Vec<_>>(),
                        "text":step["text"],

                    "doc":step.pointer("/argument/docString/content"),
                        "table":step
                            .pointer("/argument/dataTable/rows")
                            .map(|rows|rows
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|row|row
                            ["cells"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|cell|cell
                            ["value"]
                            .clone())
                            .collect::<Vec<_>>())
                            .collect::<Vec<_>>())
                })
            })
        })
        .collect()
}
#[then(
    "real handlers receive the official trimmed names arguments and \
        inherited tags"
)]
fn handlers(world: &mut Lexical) {
    assert_eq!(
        world.observed,
        expected_execution(
            world.official.as_ref().unwrap()["pickles"].as_array().unwrap()
        )
    );
}
fn span(
    source: &str,
    keyword: &str,
    span: gherkin::Span,
    position: gherkin::LineCol,
) {
    assert!(span.start < span.end && span.end <= source.len());
    assert!(source[span.start..span.end].starts_with(keyword));
    let prefix = &source[..span.start];
    assert_eq!(
        position.line,
        prefix.bytes().filter(|byte| *byte == b'\n').count() + 1
    );
    assert_eq!(
        position.col,
        prefix.rsplit('\n').next().unwrap().chars().count() + 1
    );
}
#[then(
    "captured source bytes and native byte spans still identify the original \
        declarations"
)]
fn spans(world: &mut Lexical) {
    let snapshot = world.store.get(&world.path).unwrap();
    assert_eq!(snapshot.source(), world.source);
    assert_eq!(
        snapshot.lexical_semantics(),
        gherkin::LexicalSemantics::Gherkin43
    );
    let feature = snapshot.feature();
    span(&world.source, &feature.keyword, feature.span, feature.position);
    let background = feature.background.as_ref().unwrap();
    span(
        &world.source,
        &background.keyword,
        background.span,
        background.position,
    );
    let rule = &feature.rules[0];
    span(&world.source, &rule.keyword, rule.span, rule.position);
    let scenario = &rule.scenarios[0];
    span(&world.source, &scenario.keyword, scenario.span, scenario.position);
    for step in &scenario.steps {
        span(&world.source, &step.keyword, step.span, step.position);
    }
    for example in &scenario.examples {
        span(&world.source, &example.keyword, example.span, example.position);
    }
    assert_eq!(reported(world, "source")[0]["data"], world.source);
}
#[then("selecting the actual padded Examples body row executes only that row")]
async fn row_selection(world: &mut Lexical) {
    let line = world
        .source
        .lines()
        .position(|line| line.contains("| blue |"))
        .unwrap()
        + 1;
    assert_eq!(
        world.store.get(&world.path).unwrap().scenario_origins().unwrap()[1]
            .example_line(),
        Some(line)
    );
    let expected = expected_execution(
        &world.official.as_ref().unwrap()["pickles"].as_array().unwrap()[1..2],
    );
    run(world, Some(LineSelectors::new().lines(&world.path, [line]).unwrap()))
        .await;
    assert_eq!(world.observed, expected);
    assert_eq!(reported(world, "testCase").len(), 1);
}
#[then("both reject it without a successful run or invented source nodes")]
fn rejected(world: &mut Lexical) {
    assert!(world.official_error);
    assert_eq!(reported(world, "parseError").len(), 1);
    assert_eq!(reported(world, "testRunFinished")[0]["success"], false);
    assert!(reported(world, "gherkinDocument").is_empty());
    assert!(reported(world, "pickle").is_empty());
    assert!(
        SourceGraph::parse_gherkin43("official.feature", &world.source, "en")
            .is_err()
    );
    schema(world);
}
#[then("both produce an empty document with the exact captured source")]
fn empty(world: &mut Lexical) {
    assert!(!world.official_error);
    graphs(world);
    let snapshot = world.store.empty_document(&world.path).unwrap();
    assert_eq!(
        snapshot.lexical_semantics(),
        gherkin::LexicalSemantics::Gherkin43
    );
    assert_eq!(snapshot.source(), world.source);
    assert_eq!(reported(world, "source")[0]["data"], world.source);
    assert!(reported(world, "testCase").is_empty());
    assert!(world.observed.is_empty());
}
#[when("it is parsed with the default legacy lexical policy")]
fn legacy_parse(world: &mut Lexical) {
    match gherkin::Feature::parse(&world.source, Default::default()) {
        Ok(feature) => world.legacy = Some(feature),
        Err(_) => world.legacy_error = true,
    }
}
#[then("legacy parsing still rejects the Unicode token indentation")]
fn legacy_rejected(world: &mut Lexical) {
    assert!(world.legacy_error);
}
#[then("native legacy values retain their previous Rust trimming behavior")]
fn legacy_trim(world: &mut Lexical) {
    let feature = world.legacy.as_ref().unwrap();
    assert_eq!(feature.name, "F");
    assert_eq!(feature.scenarios[0].name, "N");
    assert_eq!(feature.scenarios[0].steps[0].value, "value");
}
#[when(
    "the immutable source snapshot is retained and the file is replaced then \
        deleted"
)]
async fn generations(world: &mut Lexical) {
    execute(world).await;
    let original = world.store.get(&world.path).unwrap_or_else(|| {
        panic!("native source not captured: {:#?}", world.messages)
    });
    let origins = original.scenario_origins().unwrap();
    std::fs::write(
        &world.path,
        "Feature: Replacement\n Scenario: replacement\n  Given work\n",
    )
    .unwrap();
    let parsed = parser::Basic::new()
        .with_source_store(world.store.clone())
        .parse(world.path.clone(), Default::default())
        .collect::<Vec<_>>()
        .await;
    assert!(parsed[0].is_ok());
    assert_eq!(
        world.store.get(&world.path).unwrap().lexical_semantics(),
        gherkin::LexicalSemantics::Legacy
    );
    std::fs::remove_file(&world.path).unwrap();
    let parsed = parser::Basic::new()
        .with_source_store(world.store.clone())
        .parse(world.path.clone(), Default::default())
        .collect::<Vec<_>>()
        .await;
    assert!(parsed[0].is_err());
    assert!(world.store.get(&world.path).is_none());
    assert_eq!(original.source(), world.source);
    assert_eq!(original.scenario_origins().unwrap(), origins);
    assert_eq!(
        original.lexical_semantics(),
        gherkin::LexicalSemantics::Gherkin43
    );
    let graph =
        SourceGraph::from_snapshot("official.feature", &original).unwrap();
    same_graph(
        world.official.as_ref().unwrap(),
        json!({
        "gherkinDocument":graph.gherkin_document(),
        "pickles":graph.pickles()}
        ),
    );
    world.generations_ok = true;
}
#[then(
    "retained source graphs and row origins still use that snapshot's \
        lexical policy"
)]
fn generations_ok(world: &mut Lexical) {
    assert!(world.generations_ok);
}
#[tokio::main]
async fn main() {
    Lexical::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/lexical_semantics")
        .await;
}
