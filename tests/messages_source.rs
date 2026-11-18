//! Phase 6 acceptance: exact official source graphs and executed outline linkage.
use std::{collections::{HashMap, HashSet}, panic::AssertUnwindSafe, path::PathBuf, sync::LazyLock};

use cucumber::{Parser as _, World as _, given, when, then, gherkin,
    parser::{self, DialectCatalog, SourceStore}, step::{Collection, Context, Matching},
    writer::{Messages, messages::source::SourceGraph}};
use futures::{FutureExt as _, StreamExt as _, future::LocalBoxFuture};
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Default, cucumber::World)]
struct GraphWorld {
    fixture: String,
    source: String,
    path: PathBuf,
    store: SourceStore,
    catalogue: DialectCatalog,
    language: Option<String>,
    recorded: Vec<gherkin::Feature>,
    messages: Vec<Value>,
    expected: Vec<Value>,
    error: Option<String>,
    directory: Option<tempfile::TempDir>,
}

#[derive(Debug, Default, cucumber::World)]
struct ExecutionWorld;

struct RecordedParser(Vec<gherkin::Feature>);
impl<I> cucumber::Parser<I> for RecordedParser {
    type Cli = cucumber::cli::Empty;
    type Output = futures::stream::Iter<std::vec::IntoIter<parser::Result<gherkin::Feature>>>;
    fn parse(self, _: I, _: Self::Cli) -> Self::Output {
        futures::stream::iter(self.0.into_iter().map(Ok).collect::<Vec<_>>())
    }
}

fn accepted(_: &mut ExecutionWorld, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        if context.step.value.starts_with("invoice ") {
            assert_eq!(context.step.value, "invoice selected");
            assert_eq!(context.step.table.as_ref().unwrap().rows[1][0], "42");
            // Reporting normalization must not silently alter the native handler API.
            assert_eq!(context.step.docstring.as_deref(), Some("application/json\n        {\"id\":\"selected\"}\n"));
        }
    })
}

#[given(expr = "the official Gherkin 43 fixture {string}")]
fn official_fixture(world: &mut GraphWorld, fixture: String) {
    world.path = PathBuf::from(format!("tests/fixtures/gherkin43/{fixture}.feature")).canonicalize().unwrap();
    world.source = std::fs::read_to_string(&world.path).unwrap();
    world.fixture = fixture.clone();
    world.expected = ["ast", "pickles"].into_iter().flat_map(|kind| {
        std::fs::read_to_string(format!("tests/fixtures/gherkin43/{fixture}.feature.{kind}.ndjson")).unwrap()
            .lines().map(|line| serde_json::from_str::<Value>(line).unwrap()).collect::<Vec<_>>()
    }).collect();
}

#[given(expr = "the official localized Gherkin 43 fixture {string}")]
fn localized_fixture(world: &mut GraphWorld, fixture: String) {
    official_fixture(world, fixture);
    world.catalogue = DialectCatalog::Gherkin43;
}

async fn capture(world: &mut GraphWorld) {
    let mut parser = parser::Basic::new().with_dialect_catalog(world.catalogue).unwrap()
        .with_source_store(world.store.clone());
    if let Some(language) = &world.language { parser = parser.language(language.clone()).unwrap(); }
    let parsed = parser.parse(world.path.clone(), Default::default()).collect::<Vec<_>>().await;
    world.recorded = parsed.into_iter().map(|result| result.expect("official source must parse natively")).collect();
    assert_eq!(world.recorded.len(), 1);
}

#[when("its captured source is converted to Messages")]
async fn convert(world: &mut GraphWorld) {
    capture(world).await;
    let snapshot = world.store.get(&world.path).expect("parser captured source");
    assert_eq!(snapshot.source(), world.source);
    let graph = SourceGraph::from_snapshot(world.path.to_str().unwrap(), &snapshot).expect("source graph conversion");
    world.messages.push(serde_json::json!({"gherkinDocument": graph.gherkin_document()}));
    world.messages.extend(graph.pickles().iter().map(|p| serde_json::json!({"pickle": p})));
}

fn pair_ids(expected: &Value, actual: &Value, ids: &mut HashMap<String, String>, inverse: &mut HashSet<String>) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            assert_eq!(e.keys().collect::<Vec<_>>(), a.keys().collect::<Vec<_>>(), "field set mismatch\nexpected={expected}\nactual={actual}");
            if let Some(expected_id) = e.get("id").and_then(Value::as_str) {
                let actual_id = a["id"].as_str().unwrap();
                assert!(ids.insert(actual_id.to_owned(), expected_id.to_owned()).is_none(), "duplicate actual id {actual_id}");
                assert!(inverse.insert(expected_id.to_owned()), "duplicate expected id {expected_id}");
            }
            for (key, value) in e { pair_ids(value, &a[key], ids, inverse); }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(e.len(), a.len(), "array length mismatch\nexpected={expected}\nactual={actual}");
            for (e, a) in e.iter().zip(a) { pair_ids(e, a, ids, inverse); }
        }
        _ => {},
    }
}

fn normalize(value: &mut Value, ids: &HashMap<String, String>, uri: &str) {
    match value {
        Value::Object(object) => for (key, value) in object {
            if key == "uri" { *value = Value::String(uri.to_owned()); }
            else if key == "id" || key.ends_with("Id") {
                *value = Value::String(ids[value.as_str().unwrap()].clone());
            } else if key.ends_with("Ids") {
                for id in value.as_array_mut().unwrap() { *id = Value::String(ids[id.as_str().unwrap()].clone()); }
            } else { normalize(value, ids, uri); }
        },
        Value::Array(array) => for value in array { normalize(value, ids, uri); },
        _ => {},
    }
}

#[then("its AST and pickles equal the official fixture modulo IDs and URI")]
fn official(world: &mut GraphWorld) {
    let expected = Value::Array(world.expected.clone());
    let mut actual = Value::Array(world.messages.clone());
    let mut ids = HashMap::new();
    pair_ids(&expected, &actual, &mut ids, &mut HashSet::new());
    normalize(&mut actual, &ids, &format!("../testdata/good/{}.feature", world.fixture));
    assert_eq!(actual, expected, "all source fields, locations, types and linked IDs must match");
}

#[then("every graph envelope validates against Messages 34.2.1")]
fn schema(world: &mut GraphWorld) {
    static VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        let schema: Value = serde_json::from_str(include_str!("fixtures/messages/messages-34.2.1.schema.json")).unwrap();
        jsonschema::validator_for(&schema).unwrap()
    });
    assert!(!world.messages.is_empty());
    for message in &world.messages {
        let errors = VALIDATOR.iter_errors(message).map(|e| format!("{e} at {}", e.instance_path)).collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
}

fn temporary_source(world: &mut GraphWorld, source: &str) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("captured.feature");
    std::fs::write(&path, source).unwrap();
    world.path = path.canonicalize().unwrap();
    world.source = source.to_owned();
    world.directory = Some(directory);
}

#[given("a legacy French feature using the parser fallback without a directive")]
fn french_fallback(world: &mut GraphWorld) {
    world.language = Some("fr".to_owned());
    temporary_source(world, "Fonctionnalité: Facture\n  Scénario: traitement\n    Soit accepté\n    Quand envoyé\n    Alors terminé\n");
}

#[given("a Gherkin 43 Malayalam feature using the parser fallback without a directive")]
fn malayalam_fallback(world: &mut GraphWorld) {
    world.catalogue = DialectCatalog::Gherkin43;
    world.language = Some("ml".to_owned());
    temporary_source(world, "സവിശേഷത: റിപ്പോർട്ട്\n  രംഗം: പരിശോധന\n    നൽകിയത്ready\n    എപ്പോൾsent\n    പിന്നെcomplete\n");
}

#[given("a Gherkin 43 Chinese feature with a latest-only Rule keyword and Unicode table cells")]
fn chinese_rule(world: &mut GraphWorld) {
    world.catalogue = DialectCatalog::Gherkin43;
    temporary_source(world, "# language: zh-CN\n功能: Unicode\n  规则: 最新\n    场景: 表格\n      假如有效\n      | 名称 | 值 |\n");
}

#[given("a legacy Chinese feature containing the latest-only Rule spelling")]
fn legacy_chinese_rule(world: &mut GraphWorld) {
    chinese_rule(world);
    world.catalogue = DialectCatalog::Legacy;
}

#[then("the legacy graph retains that spelling as feature description")]
fn legacy_chinese_description(world: &mut GraphWorld) {
    let feature = &world.messages[0]["gherkinDocument"]["feature"];
    assert_eq!(feature["language"], "zh-CN");
    assert_eq!(feature["description"], "  规则: 最新");
    assert!(feature["children"][0].get("scenario").is_some());
    assert!(feature["children"][0].get("rule").is_none());
}

#[given("a Gherkin 43 French feature with wildcard and conjunction steps")]
fn localized_wildcard(world: &mut GraphWorld) {
    world.catalogue = DialectCatalog::Gherkin43;
    temporary_source(world, "# language: fr\nFonctionnalité: Types\n  Scénario: continuation\n    Soit prêt\n    * ensuite\n    Et suite\n");
}

#[then(expr = "the graph language is {string} with step keywords {string}")]
fn localized_keywords(world: &mut GraphWorld, language: String, expected: String) {
    let document = &world.messages[0]["gherkinDocument"];
    assert_eq!(document["feature"]["language"], language);
    let steps = document["feature"]["children"][0]["scenario"]["steps"].as_array().unwrap();
    let keywords = steps.iter().map(|s| s["keyword"].as_str().unwrap()).collect::<Vec<_>>().join(",");
    assert_eq!(keywords, expected);
    for pickle in world.messages.iter().filter_map(|m| m.get("pickle")) {
        assert_eq!(pickle["language"], language);
    }
}

#[then(expr = "the pickle step types are localized as {string}")]
fn localized_types(world: &mut GraphWorld, expected: String) {
    let types = world.messages.iter().filter_map(|m| m.get("pickle"))
        .flat_map(|p| p["steps"].as_array().unwrap())
        .map(|s| s["type"].as_str().unwrap()).collect::<Vec<_>>().join(",");
    assert_eq!(types, expected);
}

#[then(expr = "the graph language is {string} with a Rule keyword {string}")]
fn localized_rule(world: &mut GraphWorld, language: String, keyword: String) {
    let doc = &world.messages[0]["gherkinDocument"];
    assert_eq!(doc["feature"]["language"], language);
    assert_eq!(doc["feature"]["children"][0]["rule"]["keyword"], keyword);
    for pickle in world.messages.iter().filter_map(|m| m.get("pickle")) { assert_eq!(pickle["language"], language); }
}

#[then("its Unicode table cell columns are 9 and 14")]
fn localized_columns(world: &mut GraphWorld) {
    let cells = &world.messages[0]["gherkinDocument"]["feature"]["children"][0]["rule"]["children"][0]["scenario"]["steps"][0]["dataTable"]["rows"][0]["cells"];
    assert_eq!(cells[0]["location"]["column"], 9);
    assert_eq!(cells[1]["location"]["column"], 14);
    assert_eq!(cells[0]["value"], "名称");
    assert_eq!(cells[1]["value"], "值");
}

#[given("an English feature with a language directive inside a doc string and a later comment")]
fn literal_directive(world: &mut GraphWorld) {
    temporary_source(world, "Feature: Literal directives\n  Scenario: payload\n    Given accepted\n      \"\"\"\n      # language: fr\n      literal content\n      \"\"\"\n# language: de\n");
}

#[then("the document remains English and retains both literal directive lines")]
fn literal_directive_retained(world: &mut GraphWorld) {
    let doc = &world.messages[0]["gherkinDocument"];
    assert_eq!(doc["feature"]["language"], "en");
    assert_eq!(doc["feature"]["children"][0]["scenario"]["steps"][0]["docString"]["content"], "# language: fr\nliteral content");
    assert_eq!(doc["comments"][0]["text"], "# language: de");
}

#[given("a tagged outline inside a rule with comments between example rows")]
fn outline(world: &mut GraphWorld) {
    temporary_source(world, r#"@feature
Feature: Invoices
  Source-backed invoice reports
  Background: global setup
    Given setup
  @rule
  Rule: invoice policy
    Rule description
    Background: rule setup
      Given setup
    @scenario
    Scenario Outline: invoice <id>
      Given invoice <id>
        | amount |
        | <amount> |
        """application/json
        {"id":"<id>"}
        """
      @examples
      Examples: selected rows
        | id | amount |
        | ignored | 1 |
        # The selected row is not at the native synthetic line.

        | selected | 42 |
"#);
}

#[when("only the selected expanded scenario runs with captured-source reporting")]
async fn selected(world: &mut GraphWorld) {
    let mut output = Vec::new();
    let definitions = Collection::new().given(None, Regex::new("^.*$").unwrap(), accepted);
    ExecutionWorld::cucumber().steps(definitions).step_matching(Matching::KeywordIndependent)
        .with_messages(&mut output).with_default_cli()
        .filter_run(&world.path, |_, _, scenario| scenario.name == "invoice selected").await;
    world.messages = String::from_utf8(output).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
}

#[then("the report contains the complete original unexpanded AST")]
fn full_source(world: &mut GraphWorld) {
    let source = world.messages.iter().find_map(|m| m.get("source")).unwrap();
    assert_eq!(source["data"], world.source);
    let doc = world.messages.iter().find_map(|m| m.get("gherkinDocument")).unwrap();
    let rule = &doc["feature"]["children"][1]["rule"];
    assert_eq!(rule["description"], "    Rule description");
    let scenario = &rule["children"][1]["scenario"];
    assert_eq!(scenario["name"], "invoice <id>");
    assert_eq!(scenario["examples"][0]["tableBody"].as_array().unwrap().len(), 2);
    assert_eq!(doc["comments"].as_array().unwrap().len(), 1);
}

#[then("the executed test case references the selected real example row")]
fn row_link(world: &mut GraphWorld) {
    let cases = world.messages.iter().filter_map(|m| m.get("testCase")).collect::<Vec<_>>();
    assert_eq!(cases.len(), 1);
    let pickles = world.messages.iter().filter_map(|m| m.get("pickle")).collect::<Vec<_>>();
    assert_eq!(pickles.len(), 2, "captured-source graph retains filtered-out pickle");
    let selected = pickles.iter().find(|p| p["id"] == cases[0]["pickleId"]).unwrap();
    assert_eq!(selected["name"], "invoice selected");
    let line = world.source.lines().position(|line| line.contains("| selected | 42 |")).unwrap() + 1;
    assert_eq!(selected["location"]["line"], line);
    let doc = world.messages.iter().find_map(|m| m.get("gherkinDocument")).unwrap();
    let row = &doc["feature"]["children"][1]["rule"]["children"][1]["scenario"]["examples"][0]["tableBody"][1];
    assert_eq!(selected["astNodeIds"][1], row["id"]);
    assert_eq!(selected["steps"][2]["astNodeIds"][1], row["id"]);
    let tags = selected["tags"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(tags, ["@feature", "@rule", "@scenario", "@examples"]);
    assert_eq!(world.messages.last().unwrap()["testRunFinished"]["success"], true);
}

#[then("both argument values are normalized and ordered in its pickle")]
fn arguments(world: &mut GraphWorld) {
    let pickle = world.messages.iter().filter_map(|m| m.get("pickle")).find(|p| p["name"] == "invoice selected").unwrap();
    let arguments = &pickle["steps"][2]["argument"];
    assert_eq!(arguments["dataTable"]["argumentIndex"], 1);
    assert_eq!(arguments["dataTable"]["rows"][1]["cells"][0]["value"], "42");
    assert_eq!(arguments["docString"]["argumentIndex"], 2);
    assert_eq!(arguments["docString"]["mediaType"], "application/json");
    assert_eq!(arguments["docString"]["content"], r#"{"id":"selected"}"#);
}

#[given("a feature captured before its file is deleted")]
async fn deleted(world: &mut GraphWorld) {
    temporary_source(world, "Feature: Snapshot\n  Scenario: stable\n    Given accepted");
    capture(world).await;
    std::fs::remove_file(&world.path).unwrap();
}

#[given("an executed feature absent from the shared source store")]
fn absent(world: &mut GraphWorld) {
    temporary_source(world, "Feature: Snapshot\n  Scenario: stable\n    Given accepted");
    world.recorded = vec![gherkin::Feature::parse_path(&world.path, Default::default()).unwrap()];
}

#[when("its recorded native feature runs with captured-source reporting")]
async fn recorded(world: &mut GraphWorld) {
    let mut output = Vec::new();
    let execution = async {
        ExecutionWorld::cucumber::<PathBuf>().with_parser(RecordedParser(world.recorded.clone()))
            .steps(Collection::new().given(None, Regex::new("^accepted$").unwrap(), accepted))
            .with_writer(Messages::with_source_store(&mut output, world.store.clone()))
            .with_default_cli().run(&world.path).await;
    };
    world.error = AssertUnwindSafe(execution).catch_unwind().await.err().map(|error| {
        error.downcast_ref::<String>().cloned().or_else(|| error.downcast_ref::<&str>().map(|s| (*s).to_owned())).unwrap_or_default()
    });
    world.messages = String::from_utf8(output).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
}

#[then("reporting succeeds using the captured bytes without a final newline")]
fn immutable(world: &mut GraphWorld) {
    assert!(world.error.is_none(), "{:?}", world.error);
    assert!(!world.path.exists());
    assert!(!world.source.ends_with('\n'));
    assert_eq!(world.messages.iter().find_map(|m| m.get("source")).unwrap()["data"], world.source);
    assert_eq!(world.messages.last().unwrap()["testRunFinished"]["success"], true);
}

#[then("reporting rejects the missing snapshot without a successful completion")]
fn missing(world: &mut GraphWorld) {
    assert!(world.error.as_deref().is_some_and(|s| s.contains("missing captured snapshot")), "{:?}", world.error);
    assert!(!world.messages.iter().any(|m| m["testRunFinished"]["success"] == true));
}

#[tokio::main]
async fn main() {
    GraphWorld::cucumber().max_concurrent_scenarios(1).fail_on_skipped()
        .run_and_exit("tests/features/messages_source").await;
}
