//! Phase38b: original UTF16 cell columns, whole graphs and native table values.
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
    World as _,
    filter::LineSelectors,
    given,
    parser::{SourceFormat, SourceStore},
    step::{Collection, Context},
    then, when,
    writer::{
        Messages,
        messages::{ProtocolVersion, source::SourceGraph},
    },
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::{Value, json};
static OBSERVED: Mutex<Vec<Value>> = Mutex::new(Vec::new());
#[derive(Debug, Default, cucumber::World)]
struct Columns {
    source: String,
    format: SourceFormat,
    path: PathBuf,
    directory: Option<tempfile::TempDir>,
    store: SourceStore,
    official: Value,
    messages: Vec<Value>,
    observed: Vec<Value>,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution {
    name: String,
}
fn record(world: &mut Execution, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        OBSERVED.lock().unwrap().push(json!({
        "name":world.name,
        "text":context.step.value,
        "table":context.step.table.map(|table|table.rows)}
        ));
    })
}
#[given(expr = "a plain step table containing {string}")]
fn step_table(world: &mut Columns, case: String) {
    let rows = match case.as_str() {
        "emoji before a following cell" => {
            "  | 🦀 | tail |\n  | kept | cells |\n"
        }
        "multiple astral cells" => {
            "  | 🦀😀 | tail🌍 | third |\n  | kept | two | three |\n"
        }
        "escaped pipe beside emoji" => {
            "  | 🦀\\|slice | tail |\n  | kept | cells |\n"
        }
        "escaped backslash before emoji" => {
            "  | \\\\🦀 | tail |\n  | kept | cells |\n"
        }
        "unknown escape before emoji" => {
            "  | \\🦀 | tail |\n  | kept | cells |\n"
        }
        "combining characters beside emoji" => {
            "  | e\u{301}🦀 | tail |\n  | kept | cells |\n"
        }
        "NBSP cell padding and emoji" => {
            "  | \u{a0}🦀\u{a0} | tail |\n  | kept | cells |\n"
        }
        "EM SPACE preserved inside a cell" => {
            "  | \u{2003}🦀\u{2003} | tail |\n  | kept | cells |\n"
        }
        _ => panic!("unknown table case {case}"),
    };
    world.source =
        format!("Feature: Columns\n Scenario: values\n  Given values\n{rows}");
}
#[given("an outline with astral Examples headers and values")]
fn outline(world: &mut Columns) {
    world.source = "Feature: Columns\n Scenario Outline: case <🦀> <next>\n  \
        Given value <🦀>\n    | <🦀> | <next> |\n Examples:\n    | 🦀 | next \
        |\n    | 😀 | tail |\n    | 🌍🌍 | end |\n"
        .into();
}
#[given(
    "a table with NBSP indentation and astral content before its second cell"
)]
fn unicode_indent(world: &mut Columns) {
    world.source = "Feature: Columns\n Scenario: values\n  Given \
        values\n\u{a0}\u{a0}| 🦀 | tail |\n\u{a0}\u{a0}| kept | cells |\n"
        .into();
}
#[given("a Markdown table with astral content before its second cell")]
fn markdown(world: &mut Columns) {
    world.format = SourceFormat::Markdown;
    world.source =
        "# Feature: Columns\n## Scenario: values\n* Given values\n  \
        | 🦀 | tail |\n  | kept | cells |\n"
            .into();
}
fn oracle(world: &Columns) -> Value {
    let script = if world.format == SourceFormat::Markdown {
        "tests/interop/gherkin43/markdown-oracle.mjs"
    } else {
        "tests/interop/gherkin43/oracle.mjs"
    };
    let mut child = oracle::command(script)
        .args(["official.feature"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pinned Gherkin43 oracle installed");
    child.stdin.take().unwrap().write_all(world.source.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "official source rejection: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
async fn run(world: &mut Columns, selectors: Option<LineSelectors>) {
    OBSERVED.lock().unwrap().clear();
    let mut output = Vec::new();
    let app = Execution::cucumber()
        .steps(Collection::new().given(
            None,
            Regex::new("(?s)^.*$").unwrap(),
            record,
        ))
        .max_concurrent_scenarios(1)
        .with_gherkin43()
        .unwrap()
        .with_source_format(world.format)
        .with_source_store(world.store.clone())
        .before(|_, _, scenario, world| {
            Box::pin(async move {
                world.name = scenario.name.clone();
            })
        });
    let app = if let Some(selectors) = selectors {
        app.with_line_selectors(selectors)
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
#[when("the source is compiled and executed with captured Messages")]
async fn execute(world: &mut Columns) {
    world.official = oracle(world);
    let directory = tempfile::tempdir().unwrap();
    world.path = directory.path().canonicalize().unwrap().join(
        if world.format == SourceFormat::Markdown {
            "columns.feature.md"
        } else {
            "columns.feature"
        },
    );
    std::fs::write(&world.path, &world.source).unwrap();
    world.directory = Some(directory);
    run(world, None).await;
}
fn envelopes(world: &Columns, key: &str) -> Vec<Value> {
    world
        .messages
        .iter()
        .filter_map(|message| message.get(key).cloned())
        .collect()
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
        "all fields, columns, values and source references must agree"
    );
}
fn schema(world: &Columns) {
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
#[then("the full AST and Pickles equal the pinned official Gherkin43 compiler")]
fn graphs(world: &mut Columns) {
    let documents = envelopes(world, "gherkinDocument");
    assert_eq!(documents.len(), 1, "native envelopes: {:#?}", world.messages);
    same_graph(
        &world.official,
        json!({
        "gherkinDocument":documents[0],
        "pickles":envelopes(world,
        "pickle")}
        ),
    );
    let direct = if world.format == SourceFormat::Markdown {
        SourceGraph::parse_markdown_gherkin43(
            "official.feature",
            &world.source,
            "en",
        )
    } else {
        SourceGraph::parse_gherkin43("official.feature", &world.source, "en")
    }
    .unwrap();
    same_graph(
        &world.official,
        json!({
        "gherkinDocument":direct.gherkin_document(),
        "pickles":direct.pickles()}
        ),
    );
    assert_eq!(envelopes(world, "testRunFinished")[0]["success"], true);
    schema(world);
}
fn expected_handlers(pickles: &[Value]) -> Vec<Value> {
    pickles
        .iter()
        .flat_map(|pickle| {
            pickle["steps"].as_array().unwrap().iter().map(move |step| {
                json!({
                "name":pickle["name"],
                "text":step["text"],
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
                    .collect::<Vec<_>>())}
                )
            })
        })
        .collect()
}
#[then("the real native handler receives every unchanged table cell")]
#[then("real expanded handlers receive the official names and cell values")]
fn handlers(world: &mut Columns) {
    assert_eq!(
        world.observed,
        expected_handlers(world.official["pickles"].as_array().unwrap())
    );
}
fn row_locations(value: &Value, locations: &mut Vec<(usize, usize)>) {
    match value {
        Value::Object(map) => {
            if map.contains_key("cells") && map.contains_key("location") {
                locations.push((
                    value["location"]["line"].as_u64().unwrap() as usize,
                    value["location"]["column"].as_u64().unwrap() as usize,
                ));
            }
            for child in map.values() {
                row_locations(child, locations);
            }
        }
        Value::Array(array) => {
            for child in array {
                row_locations(child, locations);
            }
        }
        _ => {}
    }
}
#[then("source bytes and row locations remain unchanged")]
fn source_rows(world: &mut Columns) {
    assert_eq!(world.store.get(&world.path).unwrap().source(), world.source);
    assert_eq!(envelopes(world, "source")[0]["data"], world.source);
    let mut actual = Vec::new();
    row_locations(&envelopes(world, "gherkinDocument")[0], &mut actual);
    actual.sort();
    let expected = world
        .source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim_start_matches(|character| {
                cucumber::gherkin::LexicalSemantics::Gherkin43
                    .is_padding(character)
            });
            trimmed.starts_with('|').then(|| {
                (index + 1, line.chars().count() - trimmed.chars().count() + 1)
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual, expected,
        "row positions remain scalar and source-faithful"
    );
}
#[then("selecting an actual Examples row still references that original row")]
async fn selected_row(world: &mut Columns) {
    let line = world
        .source
        .lines()
        .position(|line| line.contains("| 🌍🌍 | end |"))
        .unwrap()
        + 1;
    assert_eq!(
        world.store.get(&world.path).unwrap().scenario_origins().unwrap()[1]
            .example_line(),
        Some(line)
    );
    let expected =
        expected_handlers(&world.official["pickles"].as_array().unwrap()[1..2]);
    run(world, Some(LineSelectors::new().lines(&world.path, [line]).unwrap()))
        .await;
    assert_eq!(world.observed, expected);
    let documents = envelopes(world, "gherkinDocument");
    let row = &documents[0]["feature"]["children"][0]["scenario"]["examples"]
        [0]["tableBody"][1];
    assert_eq!(row["location"]["line"], line);
    let pickles = envelopes(world, "pickle");
    assert_eq!(pickles.len(), 2);
    let cases = envelopes(world, "testCase");
    assert_eq!(cases.len(), 1);
    let selected = pickles
        .iter()
        .find(|pickle| pickle["id"] == cases[0]["pickleId"])
        .unwrap();
    assert_eq!(selected["astNodeIds"][1], row["id"]);
    assert_eq!(envelopes(world, "testRunFinished")[0]["success"], true);
    schema(world);
}
#[tokio::main]
async fn main() {
    Columns::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_table_columns")
        .await;
}
