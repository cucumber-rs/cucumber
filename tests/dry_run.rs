//! Phase35 acceptance through the real native runner, Messages35 and official Query.
use cucumber::{
    Event, StatsWriter as _, Runner as _, World as _, WriterExt as _,
    attachment::{AttachmentError, AttachmentStore},
    event,
    execution::{
        Completion, ControlError, ExecutionControl, ExecutionMode, Outcome,
        OutcomePolicy, OutcomeStatus, WorldContext,
    },
    given,
    runner::{
        BackgroundSemantics, Basic, Hooks, RunHookStore, RunHooks,
        ScenarioHookDisposition, ScenarioHookSkipReason, ScenarioHookStore,
        StepHookStore, StepHooks, TimeoutClock, TimeoutPolicy, Timeouts,
    },
    step::{Collection, Context, Matching},
    then, when,
    writer::{self, Coloring},
};
use futures::future::{BoxFuture, LocalBoxFuture};
use regex::Regex;
use serde_json::Value;
use std::{
    convert::Infallible,
    io::Write as _,
    process::{Command, Stdio},
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
static MODE: LazyLock<Mutex<String>> = LazyLock::new(Default::default);
static WORLDS: AtomicUsize = AtomicUsize::new(0);
static BODIES: AtomicUsize = AtomicUsize::new(0);
static CALLBACKS: AtomicUsize = AtomicUsize::new(0);
static TIMERS: AtomicUsize = AtomicUsize::new(0);
static CAPABILITIES: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug, Default)]
struct Application {
    execution: ExecutionControl,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
    async fn new_with_runtime_context(
        context: WorldContext,
    ) -> Result<Self, Self::Error> {
        WORLDS.fetch_add(1, Ordering::SeqCst);
        let mode = MODE.lock().unwrap().clone();
        if mode == "constructor context" {
            assert_eq!(
                context.attachments().scope().unwrap_err(),
                AttachmentError::NoActiveScope
            );
            assert_eq!(
                context.execution().current().unwrap_err(),
                ControlError::NoActiveScope
            );
            CAPABILITIES.fetch_add(1, Ordering::SeqCst);
        }
        assert_ne!(
            mode, "constructor failure",
            "actual dry-run World initialization failure"
        );
        Ok(Self { execution: context.execution() })
    }
}
#[derive(Debug)]
struct Clock;
impl TimeoutClock for Clock {
    fn sleep(&self, _: Duration) -> BoxFuture<'static, ()> {
        TIMERS.fetch_add(1, Ordering::SeqCst);
        Box::pin(futures::future::pending())
    }
}
fn callback() {
    CALLBACKS.fetch_add(1, Ordering::SeqCst);
}
fn body(w: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        BODIES.fetch_add(1, Ordering::SeqCst);
        if MODE.lock().unwrap().as_str() == "restore run" {
            w.execution
                .current()
                .unwrap()
                .complete(Completion::Pending)
                .unwrap();
        }
    })
}
#[derive(Debug, Clone, Default)]
struct Recorder(Arc<Mutex<Vec<Option<Outcome>>>>);
impl writer::NonTransforming for Recorder {}
impl cucumber::Writer<Application> for Recorder {
    type Cli = cucumber::cli::Empty;
    async fn handle_event(
        &mut self,
        input: cucumber::parser::Result<Event<event::Cucumber<Application>>>,
        _: &Self::Cli,
    ) {
        let (e, meta) = input.unwrap().split();
        if let event::Cucumber::Feature(
            _,
            event::Feature::Scenario(
                _,
                event::RetryableScenario {
                    event: event::Scenario::Finished,
                    ..
                },
            ),
        ) = e
        {
            self.0.lock().unwrap().push(meta.outcome);
        }
    }
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    scenario: ScenarioHookStore,
    steps: StepHookStore,
    globals: RunHookStore,
    messages: Vec<Value>,
    query: Value,
    failed: bool,
    failed_steps: usize,
    hook_errors: usize,
    recorder: Recorder,
}
#[given(expr = "a dry-run application configured for {string}")]
fn configured(w: &mut Acceptance, mode: String) {
    w.mode = mode;
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
        for message in messages {
            writeln!(input, "{message}").unwrap();
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
#[when("the selected application is inspected")]
async fn inspect(w: &mut Acceptance) {
    *MODE.lock().unwrap() = w.mode.clone();
    for counter in [&WORLDS, &BODIES, &CALLBACKS, &TIMERS, &CAPABILITIES] {
        counter.store(0, Ordering::SeqCst);
    }
    let mut hooks = Hooks::new();
    if matches!(w.mode.as_str(), "observed hooks" | "constructor failure") {
        hooks = hooks
            .with_store(w.scenario.clone())
            .before("before-one", |_, _, _, _: &mut Application| {
                Box::pin(async {
                    callback();
                })
            })
            .before_if(
                "excluded-before",
                "@absent".parse().unwrap(),
                |_, _, _, _| {
                    Box::pin(async {
                        callback();
                    })
                },
            )
            .before("before-two", |_, _, _, _| {
                Box::pin(async {
                    callback();
                })
            })
            .after("after-one", |_, _, _, _, _| {
                Box::pin(async {
                    callback();
                })
            })
            .after("after-two", |_, _, _, _, _| {
                Box::pin(async {
                    callback();
                })
            });
    } else if w.mode == "legacy hooks" {
        hooks = hooks
            .before("replaced", |_, _, _, _: &mut Application| {
                Box::pin(async {
                    callback();
                })
            })
            .after("unobserved", |_, _, _, _, _| {
                Box::pin(async {
                    callback();
                })
            });
    }
    let globals = RunHooks::new()
        .with_store(w.globals.clone())
        .before("global-before-one", |_| {
            Box::pin(async {
                callback();
            })
        })
        .before("global-before-two", |_| {
            Box::pin(async {
                callback();
            })
        })
        .after("global-after-one", |_| {
            Box::pin(async {
                callback();
            })
        })
        .after("global-after-two", |_| {
            Box::pin(async {
                callback();
            })
        });
    let step_hooks = StepHooks::new()
        .with_store(w.steps.clone())
        .before("step-before", |_, _: &mut Application| {
            Box::pin(async {
                callback();
            })
        })
        .after("step-after", |_, _, _| {
            Box::pin(async {
                callback();
            })
        });
    let mut source =
        "Feature: Inspect application\n Scenario: selected\n".to_owned();
    match w.mode.as_str(){"empty"=>{},"undefined"=>source.push_str("  Given undefined\n"),"ambiguous"=>source.push_str("  Given ambiguous\n"),"mixed"=>source.push_str("  Given known\n  Given undefined\n  Given known\n  Given ambiguous\n"),_=>source.push_str("  Given known\n")}
    if w.mode == "selection" {
        source.push_str(" Scenario: excluded\n  Given known\n");
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dry.feature");
    std::fs::write(&path, source).unwrap();
    let sources = cucumber::parser::SourceStore::new();
    let support_code = cucumber::support_code::Store::new();
    let attachments = AttachmentStore::new();
    let mut messages = Vec::new();
    let mut output = Vec::new();
    let reporter = writer::Messages::raw_with_source_store_and_protocol(
        &mut messages,
        sources.clone(),
        writer::messages::ProtocolVersion::V35,
    )
    .with_support_code_store(support_code.clone())
    .with_scenario_hook_store(w.scenario.clone())
    .with_run_hook_store(w.globals.clone())
    .with_attachment_store(attachments.clone())
    .discard_arbitrary_writes();
    let writer = writer::Basic::raw(&mut output, Coloring::Never, 0)
        .tee::<Application, _>(reporter)
        .tee::<Application, _>(w.recorder.clone().discard_arbitrary_writes())
        .normalized()
        .summarized();
    let runner = Basic::<Application>::default().with_support_code_store(support_code);
    let runner = if w.mode == "default run" {
        runner
    } else {
        runner.with_execution_mode(ExecutionMode::DryRun)
    };
    let runner = if w.mode == "restore run" {
        runner
            .with_outcome_policy(OutcomePolicy::CucumberJs {
                strict_pending: false,
            })
            .with_execution_mode(ExecutionMode::Run)
    } else {
        runner
    };
    // The actual config survives a clone and a type-changing classifier builder.
    let runner = runner
        .clone()
        .which_scenario(|_, _, _| cucumber::runner::ScenarioType::Concurrent);
    let app = cucumber::Cucumber::new()
        .with_runner(runner)
        .with_source_store(sources)
        .with_attachments(attachments)
        .with_hooks(hooks)
        .with_step_hooks(step_hooks)
        .with_background_semantics(BackgroundSemantics::Gherkin43)
        .with_timeouts(
            Timeouts::default()
                .with_default(TimeoutPolicy::After(Duration::from_secs(1)))
                .with_clock(Arc::new(Clock)),
        )
        .steps(
            Collection::new()
                .given(None, Regex::new("^known$").unwrap(), body)
                .given(None, Regex::new("^ambiguous$").unwrap(), body)
                .when(None, Regex::new("^ambiguous$").unwrap(), body),
        )
        .step_matching(Matching::KeywordIndependent)
        .max_concurrent_scenarios(1)
        .retries(2)
        .with_writer(writer)
        .with_run_hooks(globals)
        .with_default_cli();
    let report = if w.mode == "legacy hooks" {
        app.before(|_, _, _, _| {
            Box::pin(async {
                callback();
            })
        })
        .filter_run(&path, |_, _, s| s.name == "selected")
        .await
    } else {
        app.filter_run(&path, |_, _, s| s.name == "selected").await
    };
    w.failed = report.execution_has_failed();
    w.failed_steps = report.failed_steps();
    w.hook_errors = report.hook_errors();
    assert_eq!(report.parsing_errors(), 0);
    drop(report);
    w.messages = String::from_utf8(messages)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    w.query = query(&w.messages, "tests/interop/scenario-hooks.mjs");
}
fn finished(w: &Acceptance) -> Vec<&Value> {
    w.messages.iter().filter_map(|m| m.get("testCaseFinished")).collect()
}
fn outcomes(w: &Acceptance) -> Vec<String> {
    let query = query(&w.messages, "tests/interop/consume.mjs");
    query["outcomes"][0]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r.get("text").is_some())
        .map(|r| r["status"].as_str().unwrap().to_owned())
        .collect()
}
#[then(expr = "its step outcomes are {string}")]
fn statuses(w: &mut Acceptance, expected: String) {
    assert_eq!(outcomes(w).join(","), expected);
}
#[then("inspection succeeds once without invoking callbacks or deadlines")]
fn succeeds(w: &mut Acceptance) {
    assert!(!w.failed, "{:?}", w.messages);
    assert_eq!(WORLDS.load(Ordering::SeqCst), 1);
    assert_eq!(BODIES.load(Ordering::SeqCst), 0);
    assert_eq!(CALLBACKS.load(Ordering::SeqCst), 0);
    assert_eq!(TIMERS.load(Ordering::SeqCst), 0);
    assert_eq!(finished(w).len(), 1);
    assert_eq!(finished(w)[0]["willBeRetried"], false);
    assert!(w.steps.records().is_empty());
    assert_eq!(w.query["success"], true);
    assert!(
        w.recorder.0.lock().unwrap()[0]
            .is_some_and(|o| !o.fails_run && !o.retryable)
    );
}
#[then(
    "schema-valid Messages preserve the classification and successful run in the official Query"
)]
fn schema_query(w: &mut Acceptance) {
    let schema: Value = serde_json::from_str(include_str!(
        "fixtures/messages/messages-35.0.0.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for message in &w.messages {
        let errors: Vec<_> =
            validator.iter_errors(message).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{message}: {errors:?}");
    }
    let result = query(&w.messages, "tests/interop/consume.mjs");
    assert_eq!(result["observedAttempts"], 1);
    assert_eq!(w.query["success"], true);
    if matches!(w.mode.as_str(), "mixed" | "ambiguous") {
        assert_eq!(w.failed_steps, if w.mode == "mixed" { 2 } else { 1 }, "undefined and ambiguity remain visible in counters");
        assert!(!w.failed);
    }
    let definitions: std::collections::HashSet<_> = w
        .messages
        .iter()
        .filter_map(|m| m.get("stepDefinition").and_then(|d| d["id"].as_str()))
        .collect();
    let pickle = w.messages.iter().find_map(|m| m.get("pickle")).unwrap();
    let case = w.messages.iter().find_map(|m| m.get("testCase")).unwrap();
    for step in case["testSteps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s.get("pickleStepId").is_some())
    {
        let text = pickle["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == step["pickleStepId"])
            .unwrap()["text"]
            .as_str()
            .unwrap();
        let ids = step["stepDefinitionIds"].as_array().unwrap();
        assert_eq!(
            ids.len(),
            match text {
                "known" => 1,
                "undefined" => 0,
                "ambiguous" => 2,
                _ => unreachable!(),
            }
        );
        assert!(
            ids.iter().all(|id| definitions.contains(id.as_str().unwrap()))
        );
    }
    for m in &w.messages {
        if let Some(end) = m.get("testStepFinished") {
            assert_eq!(
                end["testStepResult"]["duration"],
                serde_json::json!({"seconds":0,"nanos":0})
            );
        }
    }
}
#[then(
    "selected scenario callbacks have dry-run skip decisions and excluded callbacks are absent"
)]
fn scenario_decisions(w: &mut Acceptance) {
    let records = w.scenario.records();
    assert_eq!(records.len(), 4);
    assert_eq!(
        records.iter().map(|r| r.definition.name.as_str()).collect::<Vec<_>>(),
        vec!["before-one", "before-two", "after-two", "after-one"]
    );
    assert!(records.iter().all(|r| matches!(
        r.disposition,
        ScenarioHookDisposition::Skipped {
            reason: ScenarioHookSkipReason::DryRun,
            ..
        }
    )));
    assert_eq!(w.scenario.initializations().len(), 1);
    assert!(w.scenario.initializations()[0].result.is_ok());
}
#[then(
    "Messages contain skipped callback slots and the real native World initialization"
)]
fn scenario_messages(w: &mut Acceptance) {
    let steps = w.query["attempts"][0]["steps"].as_array().unwrap();
    let hooks: Vec<_> =
        steps.iter().filter(|r| r.get("hook").is_some()).collect();
    assert_eq!(hooks.len(), 5);
    assert_eq!(hooks[0]["hook"]["name"], "Native World initialization");
    assert_eq!(hooks[0]["status"], "PASSED");
    assert!(hooks[1..].iter().all(|r| r["status"] == "SKIPPED"));
    for r in w.scenario.records() {
        let id = format!("native-scenario-hook-{}", r.definition.id);
        assert!(hooks.iter().any(|row| row["hook"]["id"] == id));
    }
}
#[then(
    "native setup and cleanup retain skipped metadata without invocation records"
)]
fn legacy(w: &mut Acceptance) {
    assert!(w.scenario.records().is_empty());
    let rows = w.query["attempts"][0]["steps"].as_array().unwrap();
    let hooks: Vec<_> =
        rows.iter().filter(|r| r.get("hook").is_some()).collect();
    assert_eq!(hooks.len(), 2);
    assert!(hooks.iter().all(|r| r["status"] == "SKIPPED"));
}
#[then(
    "global selections are skipped in setup and reverse cleanup order without actual invocation records"
)]
fn global_decisions(w: &mut Acceptance) {
    assert!(w.globals.records().is_empty());
    let skipped = w.globals.skipped_for(w.globals.current_run().unwrap());
    assert_eq!(
        skipped.iter().map(|r| r.definition.name.as_str()).collect::<Vec<_>>(),
        vec![
            "global-before-one",
            "global-before-two",
            "global-after-two",
            "global-after-one"
        ]
    );
    let ids: std::collections::HashSet<_> =
        skipped.iter().map(|r| r.decision_id).collect();
    assert_eq!(ids.len(), 4);
}
#[then(
    "Messages contain zero-duration skipped global decisions with real definition identities"
)]
fn global_messages(w: &mut Acceptance) {
    let rows = query(&w.messages, "tests/interop/run-hooks.mjs");
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 4);
    for (row, decision) in rows.iter().zip(w.globals.skips()) {
        assert_eq!(
            row["hook"]["id"],
            format!("native-run-hook-{}", decision.definition.id)
        );
        assert_eq!(row["hook"]["name"], decision.definition.name);
        assert_eq!(row["result"]["status"], "SKIPPED");
        assert_eq!(
            row["result"]["duration"],
            serde_json::json!({"seconds":0,"nanos":0})
        );
        let start = &row["started"];
        let finish = w
            .messages
            .iter()
            .find_map(|m| {
                m.get("testRunHookFinished")
                    .filter(|f| f["testRunHookStartedId"] == start["id"])
            })
            .unwrap();
        assert_eq!(finish["timestamp"], start["timestamp"]);
    }
}
#[then("one World is constructed and the empty attempt is skipped")]
fn empty(w: &mut Acceptance) {
    assert_eq!(WORLDS.load(Ordering::SeqCst), 1);
    assert!(outcomes(w).is_empty());
    assert_eq!(
        w.recorder.0.lock().unwrap()[0].unwrap().status,
        OutcomeStatus::Skipped
    );
}
#[then("the constructor has no active attachment or completion scope")]
fn capabilities(_: &mut Acceptance) {
    assert_eq!(CAPABILITIES.load(Ordering::SeqCst), 1);
}
#[then(
    "the actual initialization failure is reported without retry or callback invocation"
)]
fn constructor_failure(w: &mut Acceptance) {
    assert!(w.failed);
    assert_eq!(WORLDS.load(Ordering::SeqCst), 1);
    assert_eq!(BODIES.load(Ordering::SeqCst), 0);
    assert_eq!(CALLBACKS.load(Ordering::SeqCst), 0);
    assert_eq!(TIMERS.load(Ordering::SeqCst), 0);
    assert_eq!(finished(w).len(), 1);
    assert_eq!(finished(w)[0]["willBeRetried"], false);
    let init = w.scenario.initializations();
    assert_eq!(init.len(), 1);
    assert!(
        init[0]
            .result
            .as_ref()
            .unwrap_err()
            .contains("actual dry-run World initialization failure")
    );
    let outcome = w.recorder.0.lock().unwrap()[0].unwrap();
    assert_eq!(outcome.status, OutcomeStatus::Failed);
    assert!(!outcome.retryable);
    assert_eq!(w.query["success"], false);
}
#[then("only the selected scenario constructs a World")]
fn selected(w: &mut Acceptance) {
    assert_eq!(WORLDS.load(Ordering::SeqCst), 1);
    assert_eq!(w.query["attempts"].as_array().unwrap().len(), 1);
    assert_eq!(w.query["attempts"][0]["name"], "selected");
}
#[then(
    "ordinary invocation resumes and its configured pending outcome remains authoritative"
)]
fn restored(w: &mut Acceptance) {
    assert_eq!(BODIES.load(Ordering::SeqCst), 1);
    assert!(!w.failed);
    let o = w.recorder.0.lock().unwrap()[0].unwrap();
    assert_eq!(o.status, OutcomeStatus::Pending);
    assert!(!o.fails_run);
    assert_eq!(w.globals.records().len(), 4);
    assert!(w.globals.skips().is_empty());
}
#[then("the ordinary body executes and legacy outcome metadata stays absent")]
fn default_run(w: &mut Acceptance) {
    assert_eq!(BODIES.load(Ordering::SeqCst), 1);
    assert!(!w.failed);
    assert!(w.recorder.0.lock().unwrap()[0].is_none());
    assert_eq!(w.globals.records().len(), 4);
    assert!(w.globals.skips().is_empty());
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/dry_run")
        .await;
}
