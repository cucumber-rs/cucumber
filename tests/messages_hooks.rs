//! Phase 11 acceptance: real runner hooks, bounded native stage reporting.
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};

use cucumber::{
    World as _,
    event::ScenarioFinished,
    gherkin::{Feature, Rule, Scenario},
    given,
    runner::Hooks,
    step::{Collection, Context},
    then, when,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::{Value, json};

#[derive(Debug, Default, cucumber::World)]
struct Report {
    config: String,
    messages: Vec<Value>,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution;
#[derive(Default)]
struct Observed {
    config: String,
    trace: Vec<String>,
    attempt: usize,
}
static OBSERVED: LazyLock<Mutex<Observed>> = LazyLock::new(Default::default);
fn act(_: &mut Execution, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {
        let fail = {
            let mut state = OBSERVED.lock().unwrap();
            state.trace.push("body".into());
            state.config == "body-failure"
        };
        assert!(!fail, "deliberate body failure");
    })
}
fn noop(_: &mut Execution, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}
fn before(
    name: &'static str,
) -> impl for<'a> Fn(
    &'a Feature,
    Option<&'a Rule>,
    &'a Scenario,
    &'a mut Execution,
) -> LocalBoxFuture<'a, ()> {
    move |_, _, _, _| {
        Box::pin(async move {
            let fail = {
                let mut state = OBSERVED.lock().unwrap();
                state.trace.push(name.into());
                if name == "before-one" {
                    state.attempt += 1;
                }
                name == "before-one"
                    && (state.config == "before-failure"
                        || (state.config == "before-retry"
                            && state.attempt == 1))
            };
            assert!(!fail, "deliberate setup failure");
        })
    }
}
fn after(
    name: &'static str,
) -> impl for<'a> Fn(
    &'a Feature,
    Option<&'a Rule>,
    &'a Scenario,
    &'a ScenarioFinished,
    Option<&'a mut Execution>,
) -> LocalBoxFuture<'a, ()> {
    move |_, _, _, _, _| {
        Box::pin(async move {
            let fail = {
                let mut state = OBSERVED.lock().unwrap();
                state.trace.push(name.into());
                state.config == "after-failure"
                    || (state.config == "after-retry" && state.attempt == 1)
            };
            assert!(!fail, "deliberate cleanup failure");
        })
    }
}
#[given(expr = "a Messages hook run configured as {string}")]
fn configure(report: &mut Report, config: String) {
    report.config = config;
}
#[when(expr = "the hooked feature executes with retry budget {int}")]
async fn execute(report: &mut Report, budget: usize) {
    *OBSERVED.lock().unwrap() =
        Observed { config: report.config.clone(), ..Observed::default() };
    let mut hooks = Hooks::new();
    if report.config == "filtered" {
        hooks = hooks
            .before_if(
                "before-one",
                "@selected".parse().unwrap(),
                before("before-one"),
            )
            .before_if(
                "before-two",
                "@selected".parse().unwrap(),
                before("before-two"),
            )
            .after_if(
                "after-one",
                "@selected".parse().unwrap(),
                after("after-one"),
            )
            .after_if(
                "after-two",
                "@selected".parse().unwrap(),
                after("after-two"),
            );
    } else {
        hooks = hooks
            .before("before-one", before("before-one"))
            .before("before-two", before("before-two"))
            .after("after-one", after("after-one"))
            .after("after-two", after("after-two"));
    }
    let mut source =
        "Feature: Hook reporting\n  Scenario: observed stages\n".to_owned();
    if report.config != "empty" {
        source.push_str(if report.config == "undefined" {
            "    Given absent\n"
        } else {
            "    Given work\n"
        });
        source.push_str("    Then done\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hooks.feature");
    std::fs::write(&path, source).unwrap();
    let steps = Collection::new()
        .given(None, Regex::new("^work$").unwrap(), act)
        .then(None, Regex::new("^done$").unwrap(), noop);
    let mut output = Vec::new();
    Execution::cucumber()
        .steps(steps)
        .with_hooks(hooks)
        .retries(budget)
        .with_messages(&mut output)
        .with_default_cli()
        .run(&path)
        .await;
    report.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
}
fn messages<'a>(report: &'a Report, kind: &str) -> Vec<&'a Value> {
    report.messages.iter().filter_map(|m| m.get(kind)).collect()
}
#[then(expr = "hook attempt statuses are {string}")]
fn statuses(report: &mut Report, expected: String) {
    let outcomes = messages(report, "testStepFinished");
    let actual = messages(report, "testCaseStarted")
        .iter()
        .map(|start| {
            outcomes
                .iter()
                .filter(|end| end["testCaseStartedId"] == start["id"])
                .map(|end| end["testStepResult"]["status"].as_str().unwrap())
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join(";");
    assert_eq!(actual, expected);
    for outcome in outcomes {
        if outcome["testStepResult"]["status"] == "FAILED" {
            let message =
                outcome["testStepResult"]["message"].as_str().unwrap();
            assert!(
                message.contains("deliberate"),
                "failure must retain its native explanation: {message}"
            );
            if message.contains("After hooks failed") {
                assert!(
                    message.contains("after-one")
                        && message.contains("after-two")
                );
            }
        }
    }
}
#[then(expr = "hook run success is {string}")]
fn success(report: &mut Report, expected: String) {
    let ends = messages(report, "testRunFinished");
    assert_eq!(ends.len(), 1);
    assert_eq!(ends[0]["success"], expected == "true");
}
#[then(expr = "the callback trace is {string}")]
fn trace(_: &mut Report, expected: String) {
    assert_eq!(OBSERVED.lock().unwrap().trace.join(","), expected);
}
#[then("the report describes only the two native composite stages")]
fn stages(report: &mut Report) {
    let hooks = messages(report, "hook");
    assert_eq!(hooks.len(), 2);
    for (hook, kind) in
        hooks.iter().zip(["BEFORE_TEST_CASE", "AFTER_TEST_CASE"])
    {
        assert_eq!(hook["type"], kind);
        assert_eq!(hook["sourceReference"], json!({}));
        assert!(hook["name"].as_str().unwrap().contains("composite stage"));
        assert!(hook.get("tagExpression").is_none());
    }
    let cases = messages(report, "testCase");
    assert_eq!(cases.len(), 1);
    let steps = cases[0]["testSteps"].as_array().unwrap();
    assert_eq!(steps.first().unwrap()["hookId"], hooks[0]["id"]);
    assert_eq!(steps.last().unwrap()["hookId"], hooks[1]["id"]);
    for step in steps.iter().filter(|step| step.get("hookId").is_some()) {
        assert!(step.get("pickleStepId").is_none());
    }
}
#[then(expr = "retries retain one test case and stage identities with flags \
        {string}")]
fn retries(report: &mut Report, expected: String) {
    let starts = messages(report, "testCaseStarted");
    assert_eq!(starts.len(), 2);
    assert_eq!(starts[0]["attempt"], 0);
    assert_eq!(starts[1]["attempt"], 1);
    assert_eq!(starts[0]["testCaseId"], starts[1]["testCaseId"]);
    assert_ne!(starts[0]["id"], starts[1]["id"]);
    let ends = messages(report, "testCaseFinished");
    assert_eq!(
        ends.iter()
            .map(|end| end["willBeRetried"].to_string())
            .collect::<Vec<_>>()
            .join(","),
        expected
    );
    let outcomes = messages(report, "testStepFinished");
    let ids = |id: &Value| {
        outcomes
            .iter()
            .filter(|end| &end["testCaseStartedId"] == id)
            .map(|end| end["testStepId"].as_str().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&starts[0]["id"]), ids(&starts[1]["id"]));
    let state = OBSERVED.lock().unwrap();
    assert_eq!(state.trace.iter().filter(|s| *s == "after-one").count(), 2);
    assert_eq!(state.trace.iter().filter(|s| *s == "after-two").count(), 2);
}
fn collect_ids(value: &Value, ids: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(id) = map.get("id") {
                assert!(
                    ids.insert(id.as_str().unwrap().into()),
                    "duplicate ID {id}"
                );
            }
            for child in map.values() {
                collect_ids(child, ids);
            }
        }
        Value::Array(array) => {
            for child in array {
                collect_ids(child, ids);
            }
        }
        _ => {}
    }
}
fn refs(value: &Value, ids: &HashSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key.ends_with("Id") {
                    assert!(
                        ids.contains(child.as_str().unwrap()),
                        "reference before definition {key}: {child}"
                    );
                } else if key.ends_with("Ids") {
                    for id in child.as_array().unwrap() {
                        assert!(ids.contains(id.as_str().unwrap()));
                    }
                } else {
                    refs(child, ids);
                }
            }
        }
        Value::Array(array) => {
            for child in array {
                refs(child, ids);
            }
        }
        _ => {}
    }
}
fn nanos(value: &Value) -> u128 {
    value["seconds"].as_u64().unwrap() as u128 * 1_000_000_000
        + value["nanos"].as_u64().unwrap() as u128
}
#[then("hook envelopes satisfy the pinned schema and ordered reference graph")]
fn schema(report: &mut Report) {
    static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
        jsonschema::validator_for(
            &serde_json::from_str(include_str!(
                "fixtures/messages/messages-34.2.1.schema.json"
            ))
            .unwrap(),
        )
        .unwrap()
    });
    let mut ids = HashSet::new();
    let mut started: Option<&Value> = None;
    let mut next = 0;
    let cases = messages(report, "testCase");
    let steps = cases[0]["testSteps"].as_array().unwrap();
    for message in &report.messages {
        let errors = SCHEMA
            .iter_errors(message)
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
        collect_ids(message, &mut ids);
        refs(message, &ids);
        if message.get("testCaseStarted").is_some() {
            next = 0;
        }
        if let Some(start) = message.get("testStepStarted") {
            assert!(started.is_none());
            assert_eq!(start["testStepId"], steps[next]["id"]);
            started = Some(start);
        }
        if let Some(end) = message.get("testStepFinished") {
            let start = started.take().unwrap();
            assert_eq!(end["testStepId"], start["testStepId"]);
            assert_eq!(end["testCaseStartedId"], start["testCaseStartedId"]);
            assert_eq!(
                nanos(&end["timestamp"]) - nanos(&start["timestamp"]),
                nanos(&end["testStepResult"]["duration"])
            );
            next += 1;
        }
        if message.get("testCaseFinished").is_some() {
            assert!(started.is_none());
            assert_eq!(next, steps.len());
        }
    }
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_hooks")
        .await;
}
