//! Phase31 actual per-callback protocol lifecycle and evidence navigation.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    convert::Infallible,
    io::Write as _,
    process::Stdio,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use cucumber::{
    World as _, WriterExt as _,
    attachment::{
        Attachment, AttachmentContext, AttachmentError, AttachmentHandle,
        AttachmentStore,
    },
    given,
    runner::{
        EmptyScenarioHooks, Hooks, ScenarioHookDisposition, ScenarioHookScope,
        ScenarioHookStore,
    },
    step::{Collection, Context},
    then, when,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;
static MODE: LazyLock<Mutex<String>> = LazyLock::new(Default::default);
static GATE: LazyLock<Mutex<Option<Arc<tokio::sync::Barrier>>>> =
    LazyLock::new(Default::default);
static REJECTIONS: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug, Default)]
struct Application {
    context: AttachmentContext,
    mode: String,
    name: String,
    attempt: usize,
    previous: Option<AttachmentHandle>,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
    async fn new_with_context(
        context: AttachmentContext,
    ) -> Result<Self, Self::Error> {
        let mode = MODE.lock().unwrap().clone();
        if matches!(
            mode.as_str(),
            "constructor attachment" | "constructor failure"
        ) {
            context
                .scope()
                .unwrap()
                .attach(Attachment::text("constructor evidence", "text/plain"))
                .unwrap();
        }
        assert_ne!(
            mode, "constructor failure",
            "actual World constructor failure"
        );
        Ok(Self { context, mode, ..Self::default() })
    }
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    messages: Vec<Value>,
    query: Value,
    store: ScenarioHookStore,
    rejections: usize,
}
fn evidence(world: &mut Application, label: &str) {
    if !matches!(
        world.mode.as_str(),
        "callback attachments" | "concurrent outline"
    ) {
        return;
    }
    if let Some(previous) = &world.previous {
        assert_eq!(
            previous.attach(Attachment::text("INVALID", "text/plain")),
            Err(AttachmentError::ClosedScope)
        );
        REJECTIONS.fetch_add(1, Ordering::SeqCst);
    }
    let handle = world.context.scope().unwrap();
    handle
        .attach(Attachment::text(
            format!("{}:{}:{label}", world.name, world.attempt),
            "text/plain",
        ))
        .unwrap();
    world.previous = Some(handle);
}
fn before<'a>(
    scope: &'a ScenarioHookScope,
    world: &'a mut Application,
) -> LocalBoxFuture<'a, ()> {
    Box::pin(async move {
        world.name = scope.attempt.scenario.name.clone();
        world.attempt = scope.attempt.attempt;
        evidence(world, &scope.definition.name);
        assert!(
            !(scope.definition.name == "setup-one"
                && matches!(
                    world.mode.as_str(),
                    "first setup fails"
                        | "both stages fail"
                        | "empty js \
                        setup"
                )),
            "actual setup failure"
        );
    })
}
fn after<'a>(
    scope: &'a ScenarioHookScope,
    world: Option<&'a mut Application>,
) -> LocalBoxFuture<'a, ()> {
    Box::pin(async move {
        if let Some(world) = world {
            evidence(world, &scope.definition.name);
            assert!(
                !(scope.definition.name == "cleanup-two"
                    && matches!(
                        world.mode.as_str(),
                        "cleanup fails" | "both stages fail"
                    )),
                "actual cleanup failure"
            );
        }
    })
}
fn body(world: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let gate = GATE.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
        assert!(
            !(world.mode == "recovering retry" && world.attempt == 0),
            "first attempt body failure"
        );
    })
}
fn all<'a>(messages: &'a [Value], kind: &str) -> Vec<&'a Value> {
    messages.iter().filter_map(|m| m.get(kind)).collect()
}
fn validate(messages: &[Value]) {
    let schema: Value = serde_json::from_str(include_str!(
        "fixtures/messages/messages-34.2.1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for m in messages {
        let errors =
            validator.iter_errors(m).map(|e| e.to_string()).collect::<Vec<_>>();
        assert!(errors.is_empty(), "{m}: {errors:?}");
    }
}
fn query(messages: &[Value]) -> Value {
    let mut child = oracle::command("tests/interop/scenario-hooks.mjs")
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
#[given(expr = "an observed scenario registry configured for {string}")]
fn select(world: &mut Acceptance, mode: String) {
    world.mode = mode;
}
#[when("the native attempt is reported with its scenario-hook records")]
async fn run(world: &mut Acceptance) {
    *MODE.lock().unwrap() = world.mode.clone();
    REJECTIONS.store(0, Ordering::SeqCst);
    *GATE.lock().unwrap() = (world.mode == "concurrent outline")
        .then(|| Arc::new(tokio::sync::Barrier::new(2)));
    world.store = ScenarioHookStore::new();
    let mut hooks = Hooks::new()
        .with_store(world.store.clone())
        .before_scoped("setup-one", before)
        .before_scoped("setup-two", before)
        .before_scoped_matching("excluded", "@absent".parse().unwrap(), before)
        .after_scoped("cleanup-one", after)
        .after_scoped("cleanup-two", after);
    if world.mode == "inherited tags" {
        hooks = hooks.before_scoped_matching(
            "selected-tag",
            "@feature and @rule and @scenario".parse().unwrap(),
            before,
        );
    }
    if world.mode == "empty js setup" {
        hooks = hooks.empty_scenario_policy(EmptyScenarioHooks::CucumberJs13_3);
    }
    let source = if world.mode == "concurrent outline" {
        "@feature\nFeature: Callbacks\n @rule\n Rule: Shared\n  @scenario\n  \
            Scenario Outline: row <value>\n   Given action <value>\n   \
            Examples:\n    | value |\n    | A |\n    | B |\n"
    } else if world.mode.starts_with("empty ") {
        "@feature\nFeature: Callbacks\n @rule\n Rule: Shared\n  @scenario\n  \
            Scenario: Work\n"
    } else {
        "@feature\nFeature: Callbacks\n @rule\n Rule: Shared\n  @scenario\n  \
            Scenario: Work\n   Given action\n"
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("callbacks.feature");
    std::fs::write(&path, source).unwrap();
    let sources = cucumber::parser::SourceStore::new();
    let attachments = AttachmentStore::new();
    let mut output = Vec::new();
    let writer = cucumber::writer::Messages::raw_with_source_store(
        &mut output,
        sources.clone(),
    )
    .with_scenario_hook_store(world.store.clone())
    .with_attachment_store(attachments.clone())
    .normalized();
    let runner = cucumber::Cucumber::new()
        .with_gherkin43()
        .unwrap()
        .steps(Collection::new().given(
            None,
            Regex::new("^action(?: .*)?$").unwrap(),
            body,
        ))
        .with_hooks(hooks)
        .with_attachments(attachments)
        .with_source_store(sources)
        .max_concurrent_scenarios(2)
        .retries(if world.mode == "recovering retry" { Some(1) } else { None })
        .with_writer(writer)
        .with_default_cli();
    if world.mode == "legacy stage replacement" {
        runner
            .before(|_, _, s, w| {
                w.name = s.name.clone();
                Box::pin(async {})
            })
            .run(&path)
            .await;
    } else {
        runner.run(&path).await;
    }
    world.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    validate(&world.messages);
    world.query = query(&world.messages);
    world.rejections = REJECTIONS.load(Ordering::SeqCst);
}
#[then(
    "every selected callback has a stable TestStep and real source definition"
)]
fn definitions(world: &mut Acceptance) {
    let cases = all(&world.messages, "testCase");
    let hooks = all(&world.messages, "hook");
    for plan in world.store.plans() {
        for definition in plan.before.iter().chain(&plan.after) {
            let id = format!("native-scenario-hook-{}", definition.id);
            let found = hooks.iter().find(|h| h["id"] == id).unwrap();
            assert_eq!(found["name"], definition.name);
            assert_eq!(
                found["sourceReference"]["uri"],
                definition.location.path
            );
            assert_eq!(
                found["sourceReference"]["location"]["line"].as_u64(),
                Some(definition.location.line.into())
            );
            assert!(cases.iter().any(|c| {
                c["testSteps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|s| s["hookId"] == id)
            }));
        }
    }
    assert!(!hooks.iter().any(|h| h["name"] == "excluded"));
}
fn at(value: &Value, time: SystemTime) {
    let elapsed = time.duration_since(UNIX_EPOCH).unwrap();
    assert_eq!(value["seconds"].as_u64(), Some(elapsed.as_secs()));
    assert_eq!(value["nanos"].as_u64(), Some(elapsed.subsec_nanos().into()));
}
fn result_for<'a>(
    world: &'a Acceptance,
    id: u64,
    attempt: usize,
) -> (&'a Value, &'a Value) {
    let hook = format!("native-scenario-hook-{id}");
    let testcase = all(&world.messages, "testCase")
        .into_iter()
        .find(|c| {
            c["testSteps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["hookId"] == hook)
        })
        .unwrap();
    let step = testcase["testSteps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["hookId"] == hook)
        .unwrap();
    let start = all(&world.messages, "testCaseStarted")
        .into_iter()
        .find(|s| {
            s["testCaseId"] == testcase["id"]
                && s["attempt"].as_u64() == Some(attempt as u64)
        })
        .unwrap();
    let s = all(&world.messages, "testStepStarted")
        .into_iter()
        .find(|s| {
            s["testCaseStartedId"] == start["id"]
                && s["testStepId"] == step["id"]
        })
        .unwrap();
    let e = all(&world.messages, "testStepFinished")
        .into_iter()
        .find(|s| {
            s["testCaseStartedId"] == start["id"]
                && s["testStepId"] == step["id"]
        })
        .unwrap();
    (s, e)
}
#[then("executed callbacks preserve their actual timestamps and result")]
fn executed(world: &mut Acceptance) {
    for record in world.store.records() {
        if let ScenarioHookDisposition::Executed {
            started_at,
            finished_at,
            result,
            ..
        } = record.disposition
        {
            let (s, e) =
                result_for(world, record.definition.id, record.attempt.attempt);
            at(&s["timestamp"], started_at);
            at(&e["timestamp"], finished_at);
            assert_eq!(
                e["testStepResult"]["status"],
                if result.is_ok() { "PASSED" } else { "FAILED" }
            );
            if let Err(message) = result {
                assert_eq!(e["testStepResult"]["message"], message);
            }
        }
    }
}
#[then("skipped callback slots do not claim execution or elapsed duration")]
fn skipped(world: &mut Acceptance) {
    for record in world.store.records() {
        if let ScenarioHookDisposition::Skipped { at: time, .. } =
            record.disposition
        {
            let (s, e) =
                result_for(world, record.definition.id, record.attempt.attempt);
            at(&s["timestamp"], time);
            at(&e["timestamp"], time);
            assert_eq!(e["testStepResult"]["status"], "SKIPPED");
            assert_eq!(e["testStepResult"]["duration"]["seconds"], 0);
            assert_eq!(e["testStepResult"]["duration"]["nanos"], 0);
        }
    }
}
#[then(
    expr = "the official Query resolves callback results and final success \
        {string}"
)]
fn outcomes(world: &mut Acceptance, success: String) {
    assert_eq!(world.query["success"], success == "true");
    let actual = world.query["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|a| a["steps"].as_array().unwrap())
        .filter(|s| {
            s["hook"]["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("native-scenario-hook-"))
        })
        .count();
    assert_eq!(actual, world.store.records().len());
}
#[then("one TestCase keeps stable callback slots across two attempts")]
fn retry_plan(world: &mut Acceptance) {
    assert_eq!(all(&world.messages, "testCase").len(), 1);
    assert_eq!(all(&world.messages, "testCaseStarted").len(), 2);
    let p = world.store.plans();
    assert_eq!(p.len(), 2);
    assert_eq!(
        p[0].before.iter().map(|d| d.id).collect::<Vec<_>>(),
        p[1].before.iter().map(|d| d.id).collect::<Vec<_>>()
    );
}
#[then(
    "each attempt reports its own callback outcomes and final retry decision"
)]
fn retry_outcome(world: &mut Acceptance) {
    executed(world);
    let ends = all(&world.messages, "testCaseFinished");
    assert_eq!(ends[0]["willBeRetried"], true);
    assert_eq!(ends[1]["willBeRetried"], false);
    assert_eq!(world.query["success"], true);
}
#[then(
    "the official Query retrieves each payload from its originating callback"
)]
fn attached(world: &mut Acceptance) {
    let attempts = world.query["attempts"].as_array().unwrap();
    for attempt in attempts {
        let steps = attempt["steps"].as_array().unwrap();
        let callbacks = steps
            .iter()
            .filter(|s| {
                s["hook"]["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("native-scenario-hook-"))
            })
            .collect::<Vec<_>>();
        assert_eq!(callbacks.len(), 4);
        for callback in callbacks {
            let payloads = callback["attachments"].as_array().unwrap();
            assert_eq!(payloads.len(), 1);
            let expected = format!(
                "{}:{}:{}",
                attempt["name"].as_str().unwrap(),
                attempt["attempt"],
                callback["hook"]["name"].as_str().unwrap()
            );
            assert_eq!(payloads[0]["body"], expected);
        }
    }
}
#[then("stale callback handles cannot attach to another callback")]
fn stale(world: &mut Acceptance) {
    assert_eq!(world.rejections, 3);
    assert!(
        !all(&world.messages, "attachment")
            .iter()
            .any(|a| a["body"] == "INVALID")
    );
}
#[then("each expanded row retains its own callback invocations and evidence")]
fn concurrent(world: &mut Acceptance) {
    assert_eq!(world.query["attempts"].as_array().unwrap().len(), 2);
    attached(world);
    let plans = world.store.plans();
    assert_ne!(plans[0].attempt.scenario_id, plans[1].attempt.scenario_id);
    for plan in plans {
        assert_eq!(world.store.records_for(plan.attempt.scenario_id).len(), 4);
    }
}
#[then(
    "selected registry callbacks are individual and the replaced stage is \
        composite"
)]
fn replaced(world: &mut Acceptance) {
    let hooks = all(&world.messages, "hook");
    assert_eq!(hooks.len(), 3);
    assert!(hooks.iter().any(|h| h["id"] == "native-hook-before-scenario"));
    assert!(!hooks.iter().any(|h| h["name"] == "setup-one"));
    assert_eq!(world.store.records().len(), 2);
}
#[then(
    "constructor evidence is not attributed to a callback that has not \
        started"
)]
fn constructor(world: &mut Acceptance) {
    let attempts = world.query["attempts"].as_array().unwrap();
    let steps = attempts[0]["steps"].as_array().unwrap();
    let init = steps
        .iter()
        .find(|s| s["hook"]["id"] == "native-world-initialization")
        .unwrap();
    assert_eq!(init["attachments"].as_array().unwrap().len(), 1);
    assert_eq!(init["attachments"][0]["body"], "constructor evidence");
    if world.mode == "constructor failure" {
        assert_eq!(init["status"], "FAILED");
        assert!(
            steps
                .iter()
                .filter(|s| s["hook"]["name"]
                    .as_str()
                    .is_some_and(|n| n.starts_with("setup-")))
                .all(|s| s["status"] == "SKIPPED")
        );
    }
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_scenario_hooks")
        .await;
}
