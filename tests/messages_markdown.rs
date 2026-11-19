//! Native Markdown phase28a acceptance; official oracle executes only here.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    collections::{HashMap, HashSet},
    io::Write as _,
    process::Stdio,
};

use cucumber::{
    World as _, gherkin::markdown::MarkdownDocument, given, then, when,
    writer::messages::source::SourceGraph,
};
use serde_json::{Value, json};
#[derive(Debug, Default, cucumber::World)]
struct Markdown {
    source: String,
    language: String,
    uri: String,
    expected: Option<Value>,
    document: Option<Result<MarkdownDocument, String>>,
    actual: Option<Result<Value, String>>,
}
#[given(expr = "the official Markdown fixture {string}")]
fn fixture(w: &mut Markdown, name: String) {
    let base = format!("tests/fixtures/gherkin43-markdown/good/{name}");
    w.source = std::fs::read_to_string(&base).unwrap();
    w.uri = format!("../testdata/good/{name}");
    w.language = "en".into();
    let ast: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{base}.ast.ndjson")).unwrap(),
    )
    .unwrap();
    let pickles = std::fs::read_to_string(format!("{base}.pickles.ndjson"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap()["pickle"].clone())
        .collect::<Vec<_>>();
    w.expected = Some(
        json!({"gherkinDocument":ast["gherkinDocument"],"pickles":pickles}),
    );
}
#[given(expr = "the Markdown boundary {string}")]
fn boundary(w: &mut Markdown, name: String) {
    w.language = if name == "French fallback" { "fr" } else { "en" }.into();
    w.uri = "boundary.feature.md".into();
    w.source = match name.as_str() {
        "French fallback" => {
            "# Fonctionnalité: Français\n## Scénario: agir\n \
            + Soit du café\n - Quand je bois\n * Alors bon\n"
        }

        "language-looking prose" => {
            "# Feature: English\n# language: fr\n## \
            Scenario: still English\n* Given work\n"
        }

        "bullets and Unicode columns" => {
            "# Feature: café\n## Scenario: 行動\n  \
            *  Given café\n + When 行動\n - Then done\n"
        }

        "indentation and separator rows" => {
            "# Feature: Tables\n## Scenario: \
            Rows\n* Given values\n | ignored |\n      | ignored |\n  | a | \
            b |\n  | :--- | value |\n     | c | d |\n"
        }

        "variable fences and media" => {
            "# Feature: Fences\n## Scenario: \
            Body\n* Given payload\n  ```` application/json \n    café\n  \
            ```\n  ```` suffix\n  ````\n"
        }

        "table then docstring" => {
            "# Feature: Both\n## Scenario: First \
            table\n* Given args\n  | cell |\n```text\npayload\n```\n"
        }

        "docstring then table" => {
            "# Feature: Both\n## Scenario: First doc\n* \
            Given args\n```text\npayload\n```\n  | cell |\n"
        }

        "rule backgrounds and examples" => {
            "# Feature: Backgrounds\n## \
            Background: shared\n* Given outer\n## Rule: limit\n### \
            Background: inner\n* Given inner\n### Scenario Outline: case \
            <x>\n* Then <x>\n#### Examples:\n  | x |\n  | --- |\n  | one |\n"
        }

        "blank first line" => {
            "\n# Feature: Later\n## Scenario: Work\n* Given \
            work\n"
        }

        "empty source" => "",
        "duplicate docstrings" => {
            "# Feature: Invalid\n## Scenario: \
            duplicate\n* Given args\n```\none\n```\n```\ntwo\n```\n"
        }

        "table docstring table" => {
            "# Feature: Invalid\n## Scenario: \
            duplicate\n* Given args\n  | one |\n```\npayload\n```\n  | two \
            |\n"
        }

        "inconsistent rows" => {
            "# Feature: Invalid\n## Scenario: rows\n* Given \
            args\n  | one |\n  | one | two |\n"
        }

        "unterminated fence" => {
            "# Feature: Invalid\n## Scenario: fence\n* \
            Given args\n````\n```\n"
        }

        "dangling tags" => "# Feature: Invalid\n`@orphan`\n",
        _ => panic!("unknown Markdown boundary {name}"),
    }
    .into();
}
#[when("its native Markdown document is parsed and compiled")]
fn parse(w: &mut Markdown) {
    w.document = Some(
        MarkdownDocument::parse(&w.source, &w.language)
            .map_err(|e| e.to_string()),
    );
    w.actual = Some(
        SourceGraph::parse_markdown_gherkin43(&w.uri, &w.source, &w.language)
            .map(|g| {
                json!({
                "gherkinDocument":g.gherkin_document(),
                "pickles":g.pickles()}
                )
            })
            .map_err(|e| e.to_string()),
    );
}
fn oracle(w: &Markdown) -> Result<Value, String> {
    let mut child =
        oracle::command("tests/interop/gherkin43/markdown-oracle.mjs")
            .args([&w.uri, &w.language])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("pinned oracle installed");
    child.stdin.take().unwrap().write_all(w.source.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout).unwrap())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
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
fn compare(w: &Markdown, expected: &Value) {
    let mut actual = w
        .actual
        .as_ref()
        .unwrap()
        .as_ref()
        .expect("native graph compilation")
        .clone();
    let mut map = HashMap::new();
    ids(expected, &actual, &mut map, &mut HashSet::new());
    normalize(&mut actual, &map, &w.uri);
    assert_eq!(&actual, expected, "complete graph semantics and references");
}
#[then("its entire Markdown graph equals the pinned expectation")]
fn pinned(w: &mut Markdown) {
    compare(w, w.expected.as_ref().unwrap());
}
#[then("its entire Markdown graph equals the live pinned oracle")]
fn live(w: &mut Markdown) {
    let expected = oracle(w).expect("valid boundary per official parser");
    compare(w, &expected);
}
#[then("both native Markdown and the pinned oracle reject it")]
fn rejected(w: &mut Markdown) {
    assert!(oracle(w).is_err(), "official accepted invalid boundary");
    assert!(
        w.document.as_ref().unwrap().is_err(),
        "native accepted invalid boundary"
    );
    assert!(w.actual.as_ref().unwrap().is_err());
}
#[then("its native projection retains original source positions and arguments")]
fn projection(w: &mut Markdown) {
    let d = w.document.as_ref().unwrap().as_ref().unwrap();
    let f = d.to_feature_at(&w.uri);
    assert_eq!(f.is_some(), d.feature.is_some());
    if let (Some(source), Some(native)) = (&d.feature, &f) {
        assert_eq!(source.header.position, native.position);
        assert_eq!(native.path.as_deref(), Some(std::path::Path::new(&w.uri)));
        let source_steps = source
            .background
            .iter()
            .flat_map(|b| &b.steps)
            .chain(source.scenarios.iter().flat_map(|s| &s.steps))
            .chain(source.rules.iter().flat_map(|r| {
                r.background
                    .iter()
                    .flat_map(|b| &b.steps)
                    .chain(r.scenarios.iter().flat_map(|s| &s.steps))
            }));
        let native_steps = native
            .background
            .iter()
            .flat_map(|b| &b.steps)
            .chain(native.scenarios.iter().flat_map(|s| &s.steps))
            .chain(native.rules.iter().flat_map(|r| {
                r.background
                    .iter()
                    .flat_map(|b| &b.steps)
                    .chain(r.scenarios.iter().flat_map(|s| &s.steps))
            }));
        for (s, n) in source_steps.zip(native_steps) {
            assert_eq!(s.position, n.position);
            assert_eq!(s.text, n.value);
            assert_eq!(
                s.doc_string.as_ref().map(|d| &d.content),
                n.docstring.as_ref()
            );
            assert_eq!(
                s.table
                    .iter()
                    .map(|r| r
                        .cells
                        .iter()
                        .map(|c| c.value.clone())
                        .collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
                n.table.as_ref().map(|t| t.rows.clone()).unwrap_or_default()
            );
        }
    }
}
#[then(
    "the parser API returns an owned native feature without a reporting \
        dependency"
)]
fn standalone(w: &mut Markdown) {
    let f =
        MarkdownDocument::parse(&w.source, "en").unwrap().to_feature().unwrap();
    assert_eq!(f.scenarios[0].steps[0].value, "the minimalism");
}
#[tokio::main]
async fn main() {
    Markdown::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_markdown")
        .await;
}
