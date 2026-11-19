//! Phase19: complete pinned plaintext good/bad corpus, no reduced comparisons.
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::LazyLock,
};

use cucumber::{
    World as _, gherkin, given, then, when,
    writer::messages::source::SourceGraph,
};
use serde_json::{Value, json};
#[derive(Debug, Default, cucumber::World)]
struct Corpus {
    fixture: String,
    kind: String,
    source: String,
    expected: Value,
    native: Option<Result<gherkin::Document, String>>,
    actual: Option<Result<Value, String>>,
}
fn load(world: &mut Corpus, kind: &str, fixture: String) {
    let path = PathBuf::from("tests/fixtures/gherkin43-corpus")
        .join(kind)
        .join(&fixture);
    world.source = std::fs::read_to_string(&path).unwrap();
    world.fixture = fixture;
    world.kind = kind.into();
    if kind == "good" {
        let envelopes = ["ast", "pickles"]
            .into_iter()
            .flat_map(|suffix| {
                std::fs::read_to_string(format!(
                    "{}.{}.ndjson",
                    path.display(),
                    suffix
                ))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str::<Value>(line).unwrap())
                .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        world.expected = json!({
        "gherkinDocument":envelopes
            .iter()
            .find_map(|e|e
            .get("gherkinDocument"))
            .unwrap(),

        "pickles":envelopes
            .iter()
            .filter_map(|e|e
            .get("pickle"))
            .collect::<Vec<_>>()}
            );
    } else {
        let expected = std::fs::read_to_string(format!(
            "{}.errors.ndjson",
            path.display()
        ))
        .unwrap();
        assert!(
            expected.lines().any(|line| serde_json::from_str::<Value>(line)
                .unwrap()
                .get("parseError")
                .is_some()),
            "official bad fixture has parser-error expectations"
        );
    }
}
#[given(expr = "the official good plaintext fixture {string}")]
fn good(world: &mut Corpus, fixture: String) {
    load(world, "good", fixture);
}
#[given(expr = "the official bad plaintext fixture {string}")]
fn bad(world: &mut Corpus, fixture: String) {
    load(world, "bad", fixture);
}
#[when("it is processed with all explicit Gherkin 43 source policies")]
fn process(world: &mut Corpus) {
    let env = gherkin::GherkinEnv::new_latest("en")
        .unwrap()
        .with_docstring_semantics(gherkin::DocStringSemantics::Gherkin43)
        .with_table_semantics(gherkin::TableSemantics::Gherkin43);
    world.native = Some(
        gherkin::Document::parse(&world.source, env).map_err(|e| e.to_string()),
    );
    if world.kind == "good" {
        let uri = format!("../testdata/good/{}", world.fixture);
        world.actual = Some(
            SourceGraph::parse_gherkin43(&uri, &world.source, "en")
                .map(|graph| {
                    json!({
                    "gherkinDocument":graph.gherkin_document(),
                    "pickles":graph.pickles()}
                    )
                })
                .map_err(|e| e.to_string()),
        );
    }
}
fn ids(
    expected: &Value,
    actual: &Value,
    map: &mut HashMap<String, String>,
    inverse: &mut HashSet<String>,
) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            assert_eq!(
                e.keys().collect::<Vec<_>>(),
                a.keys().collect::<Vec<_>>(),
                "field set differs: expected={expected} actual={actual}"
            );
            if let Some(id) = e.get("id") {
                assert!(
                    map.insert(
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
                ids(value, &a[key], map, inverse);
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(
                e.len(),
                a.len(),
                "array size differs: expected={expected} actual={actual}"
            );
            for (e, a) in e.iter().zip(a) {
                ids(e, a, map, inverse);
            }
        }
        _ => {}
    }
}
fn normalize(value: &mut Value, map: &HashMap<String, String>, uri: &str) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "uri" {
                    *value = json!(uri);
                } else if key == "id" || key.ends_with("Id") {
                    *value = json!(map[value.as_str().unwrap()]);
                } else if key.ends_with("Ids") {
                    for id in value.as_array_mut().unwrap() {
                        *id = json!(map[id.as_str().unwrap()]);
                    }
                } else {
                    normalize(value, map, uri);
                }
            }
        }
        Value::Array(array) => {
            for item in array {
                normalize(item, map, uri);
            }
        }
        _ => {}
    }
}
#[then("the entire graph equals its official expectation modulo IDs and URI")]
fn equality(world: &mut Corpus) {
    world
        .native
        .as_ref()
        .unwrap()
        .as_ref()
        .expect("official good source parses natively");
    let mut actual = world
        .actual
        .as_ref()
        .unwrap()
        .as_ref()
        .expect("official source converts completely")
        .clone();
    let mut map = HashMap::new();
    ids(&world.expected, &actual, &mut map, &mut HashSet::new());
    normalize(
        &mut actual,
        &map,
        &format!("../testdata/good/{}", world.fixture),
    );
    assert_eq!(
        actual, world.expected,
        "all fields, locations, tags, arguments, descriptions, IDs and \
            references must match"
    );
}
#[then("native document parsing rejects that official bad fixture")]
fn rejection(world: &mut Corpus) {
    assert!(
        world.native.as_ref().unwrap().is_err(),
        "native parser accepted bad official fixture {}",
        world.fixture
    );
}
#[then("every graph envelope validates against Messages 34")]
fn schema(world: &mut Corpus) {
    static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        jsonschema::validator_for(
            &serde_json::from_str(include_str!(
                "fixtures/messages/messages-34.2.1.schema.json"
            ))
            .unwrap(),
        )
        .unwrap()
    });
    let graph = world.actual.as_ref().unwrap().as_ref().unwrap();
    let mut envelopes =
        vec![json!({"gherkinDocument":graph["gherkinDocument"]})];
    envelopes.extend(
        graph["pickles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!({"pickle":p})),
    );
    for envelope in envelopes {
        let errors = SCHEMA
            .iter_errors(&envelope)
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{envelope}: {errors:?}");
    }
}
#[tokio::main]
async fn main() {
    Corpus::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_corpus")
        .await;
}
