//! Phase45: state-sensitive Markdown tags, complete official graphs and
//! handlers.
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
    gherkin::markdown::MarkdownDocument,
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
struct Application;
fn body(_: &mut Application, context: Context) -> LocalBoxFuture<'_, ()> {
    OBSERVED.lock().unwrap().push(json!({"text":context.step.value,
    "doc":context.step.docstring,
        "table":context.step.table.as_ref().map(|t| &t.rows)}
        ));
    Box::pin(async {})
}
fn definitions() -> Collection<Application> {
    Collection::new()
        .given(None, Regex::new("^.*$").unwrap(), body)
        .when(None, Regex::new("^.*$").unwrap(), body)
        .then(None, Regex::new("^.*$").unwrap(), body)
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    source: String,
    path: PathBuf,
    directory: Option<tempfile::TempDir>,
    store: SourceStore,
    messages: Vec<Value>,
    observed: Vec<Value>,
    official: Option<Result<Value, String>>,
    direct: Option<Result<Value, String>>,
    native_rejected: bool,
}
#[given(expr = "the Markdown precedence case {string}")]
fn fixture(world: &mut Acceptance, case: String) {
    let contents = match case.as_str() {
        "first step inline tag" => {
            "# Feature: Literal\n## Scenario: First\n  - Given render \
                `@literal`\n"
        }
        "following step inline tag" => {
            "# Feature: Literal\n## Scenario: Next\n* Given ready\n + When \
                render `@literal`\n - Then literal `@other` remains\n"
        }
        "feature background inline tag" => {
            "# Feature: Literal\n## Background: Shared\n* Given background \
                `@literal`\n## Scenario: Work\n* Then action\n"
        }
        "rule background inline tag" => {
            "# Feature: Literal\n## Rule: Rule\n### Background: Shared\n* \
                Given background `@literal`\n### Scenario: Work\n* Then \
                action\n"
        }
        "data table inline tag" => {
            "# Feature: Literal\n## Scenario: Table\n* Given payload\n  | \
                `@literal` | café 😀 |\n"
        }
        "following data row inline tag" => {
            "# Feature: Literal\n## Scenario: Table\n* Given payload\n  | \
                key | value |\n  | `@literal` | a\\|b |\n"
        }
        "Examples header inline tag" => {
            "# Feature: Literal\n## Scenario Outline: Row\n* Given value \
                <`@key`>\n### Examples:\n  | `@key` |\n  | value |\n"
        }
        "Examples body inline tag" => {
            "# Feature: Literal\n## Scenario Outline: Row <key>\n* Given \
                value <key>\n  | <key> |\n### Examples:\n  | key |\n  | \
                first |\n  | `@literal` |\n"
        }
        "opening fence media inline tag" => {
            "# Feature: Literal\n## Scenario: Fence\n* Given payload\n  \
                ```application/`@literal`\n    body\n  ```\n"
        }
        "table then fence inline tag" => {
            "\
                # Feature: Literal\n\
                ## Scenario: Both\n\
                * Given payload\n\
                \x20 | `@table` |\n\
                \x20 ````application/`@media`\n\
                \x20 literal `@body`\n\
                \x20 ````\n"
        }
        "fence then table inline tag" => {
            "# Feature: Literal\n## Scenario: Both\n* Given \
                payload\n```text\nbody\n```\n  | `@table` |\n"
        }
        "docstring content inline tag" => {
            "\
                # Feature: Literal\n\
                ## Scenario: Content\n\
                * Given payload\n\
                ```text\n\
                `@literal`\n\
                * Given prose `@not-a-step`\n\
                \x20 | `@not-a-table` |\n\
                ```"
        }
        "standalone inherited tags" => {
            "`@feature`\n# Feature: Tagged\n`@rule`\n## Rule: \
                Rule\n`@scenario`\n### Scenario Outline: Row <key>\n* Given \
                value <key>\n`@examples`\n#### Examples:\n  | key |\n  | \
                `@literal` |\n"
        }
        "tag-looking declaration stays tags" => {
            "# Feature: Tagged\n## Scenario: ignored `@scenario`\n## \
                Scenario: actual\n* Given action\n"
        }
        "consumed fence slot remains tags" => {
            "# Feature: Tagged\n## Scenario: First\n* Given \
                payload\n```\nbody\n```\n```application/`@next`\n## \
                Scenario: Next\n* Given action\n"
        }
        "consumed table slot remains tags" => {
            "# Feature: Tagged\n## Scenario: First\n* Given payload\n  | \
                first |\n```\nbody\n```\n  | `@next` |\n## Scenario: \
                Next\n* Given action\n"
        }
        "pending tag before literal step" => {
            "# Feature: Invalid\n## Scenario: Work\n`@pending`\n* Given \
                literal `@body`\n"
        }
        "duplicate literal docstrings" => {
            "# Feature: Invalid\n## Scenario: Work\n* Given \
                payload\n```application/`@first`\none\n```\n```application/`\
                @second`\ntwo\n```\n"
        }
        "literal table docstring table" => {
            "# Feature: Invalid\n## Scenario: Work\n* Given payload\n  | \
                `@first` |\n```\nbody\n```\n  | `@second` |\n"
        }
        "literal inconsistent rows" => {
            "# Feature: Invalid\n## Scenario: Work\n* Given payload\n  | \
                `@first` |\n  | `@second` | extra |\n"
        }
        "dangling standalone tag" => "# Feature: Invalid\n`@orphan`\n",
        _ => panic!("unknown precedence case {case}"),
    };
    let directory = tempfile::tempdir().unwrap();
    world.path =
        directory.path().canonicalize().unwrap().join("precedence.feature.md");
    world.source = contents.into();
    std::fs::write(&world.path, &world.source).unwrap();
    world.directory = Some(directory);
}
fn oracle(source: &str) -> Result<Value, String> {
    let mut child =
        oracle::command("tests/interop/gherkin43/markdown-oracle.mjs")
            .args(["official.feature.md", "en"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("pinned Markdown oracle installed");
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout).unwrap())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }
}
#[when("its captured Markdown document executes against the pinned compiler")]
async fn execute(world: &mut Acceptance) {
    world.official = Some(oracle(&world.source));
    world.native_rejected =
        MarkdownDocument::parse(&world.source, "en").is_err();
    world.direct = Some(
        SourceGraph::parse_markdown_gherkin43(
            "official.feature.md",
            &world.source,
            "en",
        )
        .map(|g| {
            json!({
            "gherkinDocument":g.gherkin_document(),
            "pickles":g.pickles()}
            )
        })
        .map_err(|e| e.to_string()),
    );
    OBSERVED.lock().unwrap().clear();
    let mut output = Vec::new();
    Application::cucumber()
        .steps(definitions())
        .max_concurrent_scenarios(1)
        .with_gherkin43()
        .unwrap()
        .with_source_format(SourceFormat::Markdown)
        .with_source_store(world.store.clone())
        .with_writer(Messages::with_source_store_and_protocol::<Application>(
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
fn envelopes<'a>(world: &'a Acceptance, key: &str) -> Vec<&'a Value> {
    world.messages.iter().filter_map(|message| message.get(key)).collect()
}
fn map_ids(
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
                "fields: {expected} / {actual}"
            );
            if let Some(id) = e.get("id") {
                assert!(
                    ids.insert(
                        a["id"].as_str().unwrap().into(),
                        id.as_str().unwrap().into()
                    )
                    .is_none(),
                    "duplicate actual ID"
                );
                assert!(
                    inverse.insert(id.as_str().unwrap().into()),
                    "duplicate expected ID"
                );
            }
            for (key, value) in e {
                map_ids(value, &a[key], ids, inverse);
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(
                e.len(),
                a.len(),
                "array lengths: {expected} / {actual}"
            );
            for (e, a) in e.iter().zip(a) {
                map_ids(e, a, ids, inverse);
            }
        }
        _ => {}
    }
}
fn normalize(value: &mut Value, ids: &HashMap<String, String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "uri" {
                    *value = json!("official.feature.md");
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
fn compare(expected: &Value, mut actual: Value) {
    let mut ids = HashMap::new();
    map_ids(expected, &actual, &mut ids, &mut HashSet::new());
    normalize(&mut actual, &ids);
    assert_eq!(
        &actual, expected,
        "complete graph fields, values, positions and references"
    );
}
#[then("its complete AST and Pickles match official Gherkin43")]
fn graph(world: &mut Acceptance) {
    let official = world
        .official
        .as_ref()
        .unwrap()
        .as_ref()
        .expect("official accepts fixture");
    compare(
        official,
        world
            .direct
            .as_ref()
            .unwrap()
            .as_ref()
            .expect("native compiler accepts fixture")
            .clone(),
    );
    let documents = envelopes(world, "gherkinDocument");
    assert_eq!(documents.len(), 1, "runtime envelopes: {:#?}", world.messages);
    compare(
        official,
        json!({
        "gherkinDocument":documents[0],
        "pickles":envelopes(world,
        "pickle")}
        ),
    );
    assert!(envelopes(world, "parseError").is_empty());
    assert_eq!(envelopes(world, "testRunFinished").len(), 1);
    assert_eq!(envelopes(world, "testRunFinished")[0]["success"], true);
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
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "schema: {message}: {errors:?}");
    }
}
#[then("actual handlers receive the official literal text and arguments")]
fn handlers(world: &mut Acceptance) {
    let official = world.official.as_ref().unwrap().as_ref().unwrap();
    let mut expected = official["pickles"].as_array().unwrap().iter()
        .flat_map(|pickle| pickle["steps"].as_array().unwrap()).map(|step|
        json!({
            "text":step["text"],
            "doc":step.pointer("/argument/docString/content"),
            
            "table":step.pointer("/argument/dataTable/rows").map(|rows| rows.as_array().unwrap().iter()
                .map(|row| row["cells"].as_array().unwrap().iter().map(|cell| cell["value"].clone()).collect::<Vec<_>>())
                .collect::<Vec<_>>())}).to_string()).collect::<Vec<_>>();
    let mut actual =
        world.observed.iter().map(Value::to_string).collect::<Vec<_>>();
    assert!(!expected.is_empty(), "fixture must exercise actual callbacks");
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected);
    assert_eq!(envelopes(world, "testStepFinished").len(), actual.len());
    assert!(
        envelopes(world, "testStepFinished")
            .iter()
            .all(|e| e["testStepResult"]["status"] == "PASSED")
    );
}
#[then("its original source bytes and typed locations are retained")]
fn retained(world: &mut Acceptance) {
    let sources = envelopes(world, "source");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["data"], world.source);
    assert_eq!(sources[0]["mediaType"], "text/x.cucumber.gherkin+markdown");
    let snapshot = world.store.get(&world.path).unwrap();
    assert_eq!(snapshot.source(), world.source);
    let document = snapshot.markdown().unwrap();
    let feature = document.feature.as_ref().unwrap();
    let steps = feature
        .background
        .iter()
        .flat_map(|b| &b.steps)
        .chain(feature.scenarios.iter().flat_map(|s| &s.steps))
        .chain(feature.rules.iter().flat_map(|r| {
            r.background
                .iter()
                .flat_map(|b| &b.steps)
                .chain(r.scenarios.iter().flat_map(|s| &s.steps))
        }));
    for step in steps {
        let line = world.source.lines().nth(step.position.line - 1).unwrap();
        assert_eq!(&world.source[step.span.start..step.span.end], line);
        assert_eq!(
            line.chars().skip(step.position.col - 1).collect::<String>(),
            format!("{}{}", step.keyword, step.text)
        );
        for row in &step.table {
            assert_eq!(
                &world.source[row.span.start..row.span.end],
                world.source.lines().nth(row.position.line - 1).unwrap()
            );
        }
    }
}
#[then("native and official parsing both reject the document")]
fn rejected(world: &mut Acceptance) {
    assert!(
        world.official.as_ref().unwrap().is_err(),
        "official accepted invalid fixture"
    );
    assert!(world.native_rejected, "native parser accepted invalid fixture");
    assert!(
        world.direct.as_ref().unwrap().is_err(),
        "native compiler accepted invalid fixture"
    );
}
#[then("no application handler runs or successful completion is reported")]
fn not_executed(world: &mut Acceptance) {
    assert!(world.observed.is_empty());
    assert_eq!(envelopes(world, "parseError").len(), 1);
    assert!(envelopes(world, "testCaseStarted").is_empty());
    assert_eq!(envelopes(world, "testRunFinished")[0]["success"], false);
    assert_eq!(envelopes(world, "source")[0]["data"], world.source);
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_markdown_precedence")
        .await;
}
