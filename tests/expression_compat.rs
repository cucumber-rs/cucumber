//! Phase 21 BDD: opt-in expression/tag semantics against pinned official JS.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    convert::Infallible,
    io::Write as _,
    process::Stdio,
    str::FromStr,
    sync::{LazyLock, Mutex},
};

use cucumber::{
    StatsWriter as _, World as _, WriterExt as _, given,
    runner::Hooks,
    then, when,
    writer::{self, Coloring},
};
use serde_json::{Value, json};
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    case: String,
    expression: String,
    text: String,
    values: Vec<Value>,
    matched: bool,
    oracle: Value,
    tags: Vec<String>,
    tag_result: Option<bool>,
    source: String,
}
#[derive(Debug, Default, cucumber::World)]
struct Application;
static VALUES: LazyLock<Mutex<Vec<Value>>> = LazyLock::new(Default::default);
static TRACE: LazyLock<Mutex<Vec<String>>> = LazyLock::new(Default::default);
fn put(value: Value) {
    VALUES.lock().unwrap().push(value);
}
#[given(expr_js = "integer {int}")]
fn integer(_: &mut Application, value: i64) {
    put(json!(value));
}
#[given(expr_js = "number {float}")]
fn number(_: &mut Application, value: f64) {
    put(json!(value));
}
#[given(expr_js = "word {word}")]
fn word(_: &mut Application, value: String) {
    put(json!(value));
}
#[given(expr_js = "text {string}")]
fn text(_: &mut Application, value: String) {
    put(json!(value));
}
#[given(expr_js = "there are {int} cucumber(s)")]
fn optional(_: &mut Application, value: i64) {
    put(json!(value));
}
#[given(expr_js = "belly/stomach has {int}")]
fn alternative(_: &mut Application, value: i64) {
    put(json!(value));
}
#[given(expr_js = r"literal \(x\) \{name\}")]
fn escaped(_: &mut Application) {}
#[given(expr_js = "anything {}")]
fn anonymous(_: &mut Application, value: String) {
    put(json!(value));
}
#[given(expr_js = "double {double}")]
fn double(_: &mut Application, value: f64) {
    put(json!(value));
}
#[given(expr_js = "decimal {bigdecimal}")]
fn decimal(_: &mut Application, value: String) {
    put(json!(value));
}
#[given(expr_js = "long {long}")]
fn long(_: &mut Application, value: i64) {
    put(json!(value));
}
#[given(expr_js = "large {biginteger}")]
fn large(_: &mut Application, value: String) {
    put(json!(value));
}
#[derive(Debug, cucumber::Parameter)]
#[param(name = "color", regex = "red|blue")]
struct Color(String);
impl FromStr for Color {
    type Err = Infallible;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.into()))
    }
}
#[given(expr_js = "color {color}")]
fn custom(_: &mut Application, value: Color) {
    put(json!(value.0));
}
#[given(expr_js = "shadow {string}")]
fn same_name(_: &mut Application, same_name: String) {
    put(json!(same_name));
}
#[given(expr = "legacy shadow {string}")]
fn old_name(_: &mut Application, old_name: String) {
    put(json!(old_name));
}
#[given(expr = "legacy number {float}")]
fn legacy_float(_: &mut Application, value: f64) {
    assert!(value.is_infinite());
    put(json!("inf"));
}
#[given(expr = "legacy text {string}")]
fn legacy_string(_: &mut Application, value: String) {
    put(json!(value));
}
#[given(expr = "selected {int}")]
fn selected(_: &mut Application, value: i64) {
    TRACE.lock().unwrap().push(format!("step:{value}"));
}

fn oracle(request: Value) -> Value {
    let mut child = oracle::command("tests/interop/expressions/oracle.mjs")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node is required for pinned official expression oracle");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "official oracle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[given(expr = "the expression compatibility case {string}")]
fn expression_case(world: &mut Acceptance, case: String) {
    let (expression, text) = match case.as_str() {
        "negative integer" => ("integer {int}", "integer -42"),
        "unsigned integer" => ("integer {int}", "integer 42"),
        "plus integer rejected" => ("integer {int}", "integer +42"),
        "unicode integer rejected" => ("integer {int}", "integer ٤٢"),
        "decimal float" => ("number {float}", "number +.5"),
        "uppercase exponent" => ("number {float}", "number 2E2"),
        "lowercase exponent rejected" => ("number {float}", "number 2e2"),
        "trailing dot rejected" => ("number {float}", "number 1."),
        "infinity rejected" => ("number {float}", "number inf"),
        "NaN rejected" => ("number {float}", "number NaN"),
        "word" => ("word {word}", "word apple"),
        "NEL word" => ("word {word}", "word a\u{0085}b"),
        "NBSP word rejected" => ("word {word}", "word a\u{00a0}b"),
        "quoted string" => ("text {string}", r#"text "hello""#),
        "escaped double quote" => ("text {string}", r#"text "a \"quote\"""#),
        "escaped single quote" => ("text {string}", r"text 'a \'quote\''"),
        "empty quoted string" => ("text {string}", r#"text """#),
        "unescaped line separator" => ("text {string}", "text \"a\u{2028}b\""),
        "escaped line separator rejected" => {
            ("text {string}", "text \"a\\\u{2028}b\"")
        }
        "optional singular" => {
            ("there are {int} cucumber(s)", "there are 1 cucumber")
        }
        "optional plural" => {
            ("there are {int} cucumber(s)", "there are 2 cucumbers")
        }
        "alternative first" => ("belly/stomach has {int}", "belly has 2"),
        "alternative second" => ("belly/stomach has {int}", "stomach has 3"),
        "escaped literal" => (r"literal \(x\) \{name\}", "literal (x) {name}"),
        "anonymous" => ("anything {}", "anything raw values"),
        "double alias" => ("double {double}", "double -1.25E2"),
        "decimal alias" => {
            ("decimal {bigdecimal}", "decimal 1234567890.1234567890")
        }
        "long alias" => ("long {long}", "long 1234567890"),
        "large integer alias" => {
            ("large {biginteger}", "large 123456789012345678901234567890")
        }
        "custom parameter" => ("color {color}", "color blue"),
        "shadowed function name" => ("shadow {string}", r#"shadow "value""#),
        "legacy shadowed function name" => {
            ("legacy shadow {string}", r#"legacy shadow "value""#)
        }
        "legacy infinity" => ("legacy number {float}", "legacy number inf"),
        "legacy escaped string" => {
            ("legacy text {string}", r#"legacy text "a \"quote\"""#)
        }
        _ => panic!("unknown case {case}"),
    };
    world.case = case;
    world.expression = expression.into();
    world.text = text.into();
}
async fn execute(world: &mut Acceptance) {
    VALUES.lock().unwrap().clear();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("expression.feature");
    std::fs::write(
        &path,
        format!(
            "Feature: Parameters\n  Scenario: Value\n    Given {}\n",
            world.text
        ),
    )
    .unwrap();
    let mut output = Vec::new();
    let result = Application::cucumber()
        .max_concurrent_scenarios(1)
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .run(&path)
        .await;
    assert_eq!(result.parsing_errors(), 0);
    assert_eq!(result.hook_errors(), 0);
    assert_eq!(result.failed_steps(), 0);
    world.matched = result.passed_steps() == 1;
    assert_eq!(result.passed_steps() + result.skipped_steps(), 1);
    world.values = VALUES.lock().unwrap().clone();
}
#[when("the step runs through an explicit JavaScript-compatible expression")]
async fn explicit(world: &mut Acceptance) {
    execute(world).await;
    world.oracle = oracle(json!({
    "kind":"expression",
    "expression":world.expression,
    "text":world.text}
    ));
}
#[when("the step runs through a legacy expression")]
async fn legacy(world: &mut Acceptance) {
    execute(world).await;
}
#[then(
    "its matched outcome and handler value equal the pinned official \
        expression implementation"
)]
fn equal_values(world: &mut Acceptance) {
    assert_eq!(world.oracle["valid"], true, "{}", world.oracle);
    assert_eq!(world.oracle["matched"], world.matched, "case {}", world.case);
    let expected = world.oracle["values"].as_array().unwrap();
    assert_eq!(world.values.len(), expected.len());
    for (actual, expected) in world.values.iter().zip(expected) {
        if actual.is_number() && expected.is_number() {
            assert_eq!(actual.as_f64(), expected.as_f64());
        } else {
            assert_eq!(actual, expected);
        }
    }
}
#[then("the legacy handler receives infinity")]
fn old_float(world: &mut Acceptance) {
    assert!(world.matched);
    assert_eq!(world.values, vec![json!("inf")]);
}
#[then("the legacy handler retains the quote escape")]
fn old_string(world: &mut Acceptance) {
    assert!(world.matched);
    assert_eq!(world.values, vec![json!(r#"a \"quote\""#)]);
}

#[given(expr = "the tag compatibility case {string}")]
fn tag_case(world: &mut Acceptance, case: String) {
    let (expression, tags): (&str, Vec<&str>) = match case.as_str() {
        "empty" => ("", vec![]),
        "whitespace" => (" \t\n", vec![]),
        "literal without at" => ("smoke", vec!["smoke"]),
        "leading trailing whitespace" => ("  @smoke  ", vec!["@smoke"]),
        "precedence" => ("@a or @b and not @c", vec!["@a", "@c"]),
        "parentheses" => ("(@a or @b) and not @c", vec!["@a", "@c"]),
        "repeated not" => ("not not @a", vec!["@a"]),
        "escaped parentheses" => (r"@a\(b\)", vec!["@a(b)"]),
        "escaped space" => (r"@a\ b", vec!["@a b"]),
        "escaped tab" => ("@a\\\tb", vec!["@a\tb"]),
        "unicode whitespace" => {
            ("\u{feff}@a\u{00a0}and\u{2003}@b", vec!["@a", "@b"])
        }
        "not prefix literal" => ("not@a", vec!["not@a"]),
        "trailing escape" => ("@a\\", vec!["@a"]),
        "invalid escape" => (r"@a\z", vec![]),
        "missing operand" => ("@a and", vec!["@a"]),
        "missing operator" => ("@a @b", vec!["@a", "@b"]),
        "unmatched parentheses" => ("(@a", vec!["@a"]),
        _ => panic!("unknown tag case"),
    };
    world.expression = expression.into();
    world.tags = tags.into_iter().map(str::to_owned).collect();
}
#[when("the explicit tag expression is parsed and evaluated")]
fn evaluate(world: &mut Acceptance) {
    world.tag_result = world
        .expression
        .parse::<cucumber::tag::Expression>()
        .ok()
        .map(|expression| expression.evaluate(&world.tags));
    world.oracle = oracle(
        json!({"kind":"tags","expression":world.expression,"tags":world.tags}),
    );
}
#[then("validity and selection equal the pinned official tag implementation")]
fn equal_tags(world: &mut Acceptance) {
    assert_eq!(
        world.oracle["valid"],
        world.tag_result.is_some(),
        "{}",
        world.oracle
    );
    if let Some(selected) = world.tag_result {
        assert_eq!(world.oracle["selected"], selected);
    }
}
#[given("an empty official tag expression")]
fn empty(world: &mut Acceptance) {
    world.expression.clear();
}
#[then("explicit official evaluation selects all tags")]
fn selects_all(world: &mut Acceptance) {
    let expression =
        world.expression.parse::<cucumber::tag::Expression>().unwrap();
    assert!(expression.evaluate(["@anything"]));
    assert!(expression.evaluate(Vec::<String>::new()));
}
#[then("legacy TagOperation parsing still rejects the empty expression")]
fn legacy_tags(world: &mut Acceptance) {
    assert!(
        world
            .expression
            .parse::<cucumber::gherkin::tagexpr::TagOperation>()
            .is_err()
    );
}
#[given("tagged feature rule and example rows")]
fn tagged(world: &mut Acceptance) {
    world.source = "\
            @feature\n\
            Feature: Selected\n\
            \x20 @rule\n\
            \x20 Rule: Inherited\n\
            \x20   Scenario Outline: Row <value>\n\
            \x20     Given selected <value>\n\
            \x20     @selected\n\
            \x20     Examples:\n\
            \x20       | value |\n\
            \x20       | 42 |\n\
            \x20     @excluded\n\
            \x20     Examples:\n\
            \x20       | value |\n\
            \x20       | 43 |\n"
        .into();
}
#[when("an explicit official expression filters the real run")]
async fn filter_run(world: &mut Acceptance) {
    TRACE.lock().unwrap().clear();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tags.feature");
    std::fs::write(&path, &world.source).unwrap();
    let expression = " @feature and @rule and @selected "
        .parse::<cucumber::tag::Expression>()
        .unwrap();
    let hooks = Hooks::new()
        .before_matching(
            "selected",
            expression.clone(),
            |_, _, _, _: &mut Application| {
                Box::pin(async {
                    TRACE.lock().unwrap().push("before".into());
                })
            },
        )
        .before_matching(
            "excluded",
            "@excluded".parse().unwrap(),
            |_, _, _, _: &mut Application| {
                Box::pin(async {
                    panic!("excluded hook must not run");
                })
            },
        )
        .after_matching("selected", expression.clone(), |_, _, _, _, _| {
            Box::pin(async {
                TRACE.lock().unwrap().push("after".into());
            })
        });
    let mut output = Vec::new();
    let result = Application::cucumber()
        .with_hooks(hooks)
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .filter_run(&path, move |f, r, s| expression.matches(f, r, s))
        .await;
    assert_eq!(result.passed_steps(), 1);
    assert_eq!(result.failed_steps(), 0);
    assert_eq!(result.skipped_steps(), 0);
    assert_eq!(result.hook_errors(), 0);
    assert_eq!(result.parsing_errors(), 0);
}
#[then("only the selected example runs and matching scenario hooks execute")]
fn selected_only(_: &mut Acceptance) {
    assert_eq!(*TRACE.lock().unwrap(), ["before", "step:42", "after"]);
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/expression_compat")
        .await;
}
