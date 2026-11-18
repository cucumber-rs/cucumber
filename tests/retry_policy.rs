//! Phase36 acceptance for validated budget/gate selection through the native runner.
use cucumber::{
    World as _,
    execution::{Completion, ExecutionControl, OutcomePolicy, WorldContext},
    given,
    runner::{Hooks, RetryPolicy, RetryPolicyError},
    step::{Collection, Context, Matching},
    then, when,
    writer::messages::ProtocolVersion,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;
use std::{
    collections::HashSet,
    convert::Infallible,
    io::Write as _,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
static CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);
static NEXT_WORLD: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug, Default)]
struct Seen {
    worlds: Vec<usize>,
    cleanup: Vec<(usize, usize)>,
    body: Vec<usize>,
}
#[derive(Debug, Default)]
struct Application {
    mode: String,
    attempt: usize,
    id: usize,
    control: ExecutionControl,
    seen: Arc<Mutex<Seen>>,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(Self {
            id: NEXT_WORLD.fetch_add(1, Ordering::SeqCst),
            ..Self::default()
        })
    }
    async fn new_with_runtime_context(
        context: WorldContext,
    ) -> Result<Self, Self::Error> {
        let mut world = Self::new().await?;
        world.control = context.execution();
        Ok(world)
    }
}
fn body(w: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        w.seen.lock().unwrap().body.push(w.attempt);
        match w.mode.as_str() {
            "pending" => {
                w.control
                    .current()
                    .unwrap()
                    .complete(Completion::Pending)
                    .unwrap();
            }
            "skipped" => {
                w.control
                    .current()
                    .unwrap()
                    .complete(Completion::Skipped)
                    .unwrap();
            }
            "feature tag" | "Rule tag" | "scenario tag" | "Examples tag"
            | "recovered" => {
                assert!(w.attempt > 0, "recoverable application failure")
            }
            _ => panic!("persistent application failure"),
        }
    })
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    seen: Arc<Mutex<Seen>>,
    messages: Vec<Value>,
    query: Value,
    invalid: Vec<RetryPolicyError>,
}
#[given("retry budget 0 and an explicitly supplied official tag gate")]
fn invalid_setup(w: &mut Acceptance) {
    CONSTRUCTIONS.store(0, Ordering::SeqCst);
    w.invalid.clear();
}
#[when("the application validates its retry policy")]
fn validate(w: &mut Acceptance) {
    for source in ["@retryable", ""] {
        w.invalid.push(
            RetryPolicy::new(0, Some(source.parse().unwrap())).unwrap_err(),
        );
    }
}
#[then(
    "validation rejects the configuration before parsing or constructing a World"
)]
fn rejected(w: &mut Acceptance) {
    assert_eq!(
        w.invalid,
        vec![RetryPolicyError::TagGateWithoutPositiveBudget; 2]
    );
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), 0);
    assert!(w.messages.is_empty());
    assert!(
        w.invalid
            .iter()
            .all(|error| error.to_string().contains("positive retry count"))
    );
}
#[given(expr = "an application with retry policy {string}")]
fn configure(w: &mut Acceptance, mode: String) {
    w.mode = mode;
    CONSTRUCTIONS.store(0, Ordering::SeqCst);
}
fn source(mode: &str) -> String {
    match mode{
        "feature tag"=>"@retryable\nFeature: Retry selection\n Scenario: application\n  Given work\n".into(),
        "Rule tag"=>"Feature: Retry selection\n @retryable\n Rule: inherited\n  Scenario: application\n   Given work\n".into(),
        "scenario tag"=>"Feature: Retry selection\n @retryable\n Scenario: application\n  Given work\n".into(),
        "Examples tag"=>"Feature: Retry selection\n Scenario Outline: application <value>\n  Given work\n  @retryable\n  Examples: inherited\n   | value |\n   | one   |\n".into(),
        "unmatched magic tag"=>"Feature: Retry selection\n @retry(7)\n Scenario: application\n  Given work\n".into(),
        "matched magic tag"=>"Feature: Retry selection\n @retryable @retry(7)\n Scenario: application\n  Given work\n".into(),
        _=>format!("Feature: Retry selection\n Scenario: application\n  Given {}\n",if mode=="undefined"{"undefined"}else{"work"}),
    }
}
fn query(messages: &[Value], script: &str) -> Value {
    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut input = child.stdin.take().unwrap();
        for m in messages {
            writeln!(input, "{m}").unwrap();
        }
    }
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
#[when("the application runs with explicit JS outcome eligibility")]
async fn run(w: &mut Acceptance) {
    let count = match w.mode.as_str() {
        "disabled" => 0,
        "exhausted" | "pending" | "ambiguous" | "undefined" | "skipped" => 2,
        _ => 1,
    };
    let gated = matches!(
        w.mode.as_str(),
        "feature tag"
            | "Rule tag"
            | "scenario tag"
            | "Examples tag"
            | "unmatched magic tag"
            | "matched magic tag"
    );
    let policy =
        RetryPolicy::new(count, gated.then(|| "@retryable".parse().unwrap()))
            .unwrap();
    let mode = w.mode.clone();
    let seen = w.seen.clone();
    let hooks = Hooks::new()
        .before_scoped("initialize", move |scope, w: &mut Application| {
            w.mode = mode.clone();
            w.attempt = scope.attempt.attempt;
            w.seen = seen.clone();
            w.seen.lock().unwrap().worlds.push(w.id);
            Box::pin(async {})
        })
        .after_scoped("cleanup", |scope, w| {
            Box::pin(async move {
                let w = w.expect("setup constructed a fresh World");
                assert_eq!(w.attempt, scope.attempt.attempt);
                w.seen.lock().unwrap().cleanup.push((w.id, w.attempt));
            })
        });
    let mut steps =
        Collection::new().given(None, Regex::new("^work$").unwrap(), body);
    if w.mode == "ambiguous" {
        steps = steps.when(None, Regex::new("^work$").unwrap(), body);
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("policy.feature");
    std::fs::write(&path, source(&w.mode)).unwrap();
    let mut output = Vec::new();
    let app = cucumber::Cucumber::new()
        .steps(steps)
        .step_matching(Matching::KeywordIndependent)
        .with_hooks(hooks)
        .retry_options(policy.into_options())
        .outcome_policy(OutcomePolicy::cucumber_js())
        .max_concurrent_scenarios(1)
        .with_messages_protocol(&mut output, ProtocolVersion::V35);
    let app = if w.mode == "custom override" {
        app.retry_options(|_, _, _, _| None)
    } else {
        app
    };
    let mut cli: cucumber::cli::Opts<
        _,
        cucumber::runner::basic::Cli,
        _,
        cucumber::cli::Empty,
    > = Default::default();
    if w.mode == "conflicting legacy CLI" {
        cli.runner.retry = Some(8);
        cli.runner.retry_tag_filter = Some("@absent".parse().unwrap());
        cli.runner.retry_after = Some(Duration::from_secs(3600));
    }
    // Bound the real runner so a delay-precedence regression is reported by BDD.
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        app.with_cli(cli).run(&path),
    )
    .await;
    assert!(
        result.is_ok(),
        "typed retry policy inherited an unexpected legacy delay"
    );
    drop(result);
    w.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    w.query = query(&w.messages, "tests/interop/consume.mjs");
}
fn messages<'a>(w: &'a Acceptance, kind: &str) -> Vec<&'a Value> {
    w.messages.iter().filter_map(|m| m.get(kind)).collect()
}
#[then(
    expr = "exactly {int} attempt is reported and its final status is {string}"
)]
#[then(
    expr = "exactly {int} attempts are reported and its final status is {string}"
)]
fn attempts(w: &mut Acceptance, count: usize, expected: String) {
    let starts = messages(w, "testCaseStarted");
    let ends = messages(w, "testCaseFinished");
    let cases = messages(w, "testCase");
    assert_eq!(cases.len(), 1);
    assert_eq!(starts.len(), count);
    assert_eq!(ends.len(), count);
    assert_eq!(CONSTRUCTIONS.load(Ordering::SeqCst), count);
    let mut ids = HashSet::new();
    for (index, (start, end)) in starts.iter().zip(ends).enumerate() {
        assert_eq!(start["attempt"], index);
        assert_eq!(start["testCaseId"], cases[0]["id"]);
        assert!(ids.insert(start["id"].as_str().unwrap()));
        assert_eq!(end["testCaseStartedId"], start["id"]);
        assert_eq!(end["willBeRetried"], index + 1 < count);
    }
    let outcomes = w.query["outcomes"].as_array().unwrap();
    // Query's final-case view intentionally excludes attempts marked for retry.
    // Raw envelopes above still verify every actual attempt and its fresh World.
    assert_eq!(w.query["observedAttempts"], count);
    assert_eq!(outcomes.len(), 1, "one terminal case remains in official Query");
    assert_eq!(outcomes[0]["attempt"], count - 1);
    assert_eq!(outcomes[0]["status"], expected);
    if matches!(w.mode.as_str(), "ambiguous" | "undefined") {
        assert!(w.seen.lock().unwrap().body.is_empty());
    }
    let successful = matches!(expected.as_str(), "PASSED" | "SKIPPED");
    assert_eq!(messages(w, "testRunFinished")[0]["success"], successful);
}
#[then("each attempt owns a fresh World and cleanup completes")]
fn fresh(w: &mut Acceptance) {
    let seen = w.seen.lock().unwrap();
    let attempts = messages(w, "testCaseStarted").len();
    assert_eq!(seen.worlds.len(), attempts);
    assert_eq!(
        seen.worlds.iter().copied().collect::<HashSet<_>>().len(),
        attempts
    );
    assert_eq!(
        seen.cleanup,
        seen.worlds
            .iter()
            .enumerate()
            .map(|(attempt, id)| (*id, attempt))
            .collect::<Vec<_>>()
    );
    assert_eq!(seen.body, (0..attempts).collect::<Vec<_>>());
}
#[then(
    expr = "schema-valid Messages and official Query retain the final success {string}"
)]
fn protocol(w: &mut Acceptance, success: String) {
    let schema: Value = serde_json::from_str(include_str!(
        "fixtures/messages/messages-35.0.0.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for message in &w.messages {
        let errors = validator
            .iter_errors(message)
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
    let observed = query(&w.messages, "tests/interop/scenario-hooks.mjs");
    assert_eq!(observed["success"], success.parse::<bool>().unwrap());
    assert_eq!(
        observed["attempts"].as_array().unwrap().len(),
        w.query["observedAttempts"].as_u64().unwrap() as usize
    );
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/retry_policy")
        .await;
}
