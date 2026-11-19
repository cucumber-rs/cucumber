//! Phase30a acceptance: actual callback deadlines, cancellation and lifecycle.
//! The driver controls timer completion through channels, never sleep
//! thresholds.
use std::{
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use cucumber::{
    StatsWriter as _, World as _, WriterExt as _,
    attachment::{
        Attachment, AttachmentContext, AttachmentError, AttachmentHandle,
        AttachmentStore,
    },
    given,
    runner::{
        Hooks, RunHookStore, RunHooks, ScenarioHookDisposition,
        ScenarioHookStore, StepHookStore, StepHooks, ThreadTimer, TimeoutClock,
        TimeoutPolicy, Timeouts,
    },
    step::{Collection, Context, DefinitionOptions},
    then, when,
};
use futures::{
    StreamExt as _,
    channel::{mpsc, oneshot},
    future::{BoxFuture, Either, LocalBoxFuture},
};
use regex::Regex;
use serde_json::Value;
const SHORT: Duration = Duration::from_millis(17);
const LONG: Duration = Duration::from_secs(19);
#[derive(Debug)]
enum Signal {
    Timer(Duration, oneshot::Sender<()>),
    Started(String, oneshot::Sender<()>),
    Done,
}
#[derive(Debug)]
struct Clock {
    tx: mpsc::UnboundedSender<Signal>,
    limits: Arc<Mutex<Vec<Duration>>>,
}
impl TimeoutClock for Clock {
    fn sleep(&self, limit: Duration) -> BoxFuture<'static, ()> {
        self.limits.lock().unwrap().push(limit);
        let (tx, rx) = oneshot::channel();
        self.tx.unbounded_send(Signal::Timer(limit, tx)).unwrap();
        Box::pin(async move {
            let _ = rx.await;
        })
    }
}
#[derive(Debug)]
struct Shared {
    mode: String,
    tx: mpsc::UnboundedSender<Signal>,
    trace: Mutex<Vec<String>>,
    handles: Mutex<Vec<AttachmentHandle>>,
    dropped: AtomicUsize,
    rejected: AtomicUsize,
    attempts: AtomicUsize,
    worlds: Mutex<Vec<usize>>,
    ticks: AtomicUsize,
}
impl Shared {
    fn trace(&self, label: impl Into<String>) {
        self.trace.lock().unwrap().push(label.into());
    }
    fn retain(&self, world: &Application) {
        let handle = world.attachments.scope().unwrap();
        handle
            .attach(Attachment::text("callback evidence", "text/plain"))
            .unwrap();
        self.handles.lock().unwrap().push(handle);
    }
    fn reject_old(&self) {
        for handle in self.handles.lock().unwrap().iter() {
            assert_eq!(
                handle.attach(Attachment::text("late", "text/plain")),
                Err(AttachmentError::ClosedScope)
            );
            self.rejected.fetch_add(1, Ordering::SeqCst);
        }
    }
}
struct DropProof(Arc<Shared>);
impl Drop for DropProof {
    fn drop(&mut self) {
        self.0.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
async fn gated(shared: Arc<Shared>, label: String) {
    let _drop = DropProof(shared.clone());
    let (tx, rx) = oneshot::channel();
    shared.tx.unbounded_send(Signal::Started(label, tx)).unwrap();
    rx.await
        .expect("driver holds blocked callbacks until release or cancellation");
}
#[derive(Debug, Default)]
struct Application {
    shared: Option<Arc<Shared>>,
    attachments: AttachmentContext,
    attempt: usize,
    id: usize,
    name: String,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Ok(Self { id: NEXT.fetch_add(1, Ordering::SeqCst), ..Self::default() })
    }
    async fn new_with_context(
        attachments: AttachmentContext,
    ) -> Result<Self, Self::Error> {
        Ok(Self { attachments, ..Self::new().await? })
    }
}
fn body(world: &mut Application, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let shared = world.shared.as_ref().unwrap().clone();
        let mode = shared.mode.as_str();
        shared.trace(format!("body:{}:{}", world.name, world.attempt));
        if matches!(mode, "cancel" | "retry") {
            let handle = world.attachments.scope().unwrap();
            handle
                .attach(Attachment::text(
                    format!("attempt:{}", world.attempt),
                    "text/plain",
                ))
                .unwrap();
            shared.handles.lock().unwrap().push(handle);
        }
        let pending = match mode {
            "cancel" | "real" | "legacy" | "preset" | "concurrent" => true,
            "retry" => world.attempt == 0,
            "longer" | "disabled" => true,
            _ => false,
        };
        if pending {
            gated(shared, format!("body:{}", context.step.value)).await;
        }
    })
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    shared: Option<Arc<Shared>>,
    limits: Vec<Duration>,
    messages: Vec<Value>,
    failed: usize,
    hooks_failed: usize,
    passed: usize,
    skipped: usize,
    failed_run: bool,
    scenario_hooks: ScenarioHookStore,
    step_hooks: StepHookStore,
    run_hooks: RunHookStore,
}
async fn drive(mut rx: mpsc::UnboundedReceiver<Signal>, shared: Arc<Shared>) {
    let mut retained = Vec::new();
    let mut short_timers = Vec::new();
    let mut started = Vec::new();
    while let Some(signal) = rx.next().await {
        match signal {
            Signal::Done => break,
            Signal::Timer(limit, tx) => {
                if limit == SHORT && shared.mode != "concurrent" {
                    let _ = tx.send(());
                } else if limit == SHORT {
                    short_timers.push(tx);
                } else {
                    retained.push(tx);
                }
            }
            Signal::Started(label, tx) => {
                shared.ticks.fetch_add(1, Ordering::SeqCst);
                match shared.mode.as_str() {
                    "legacy" | "preset" => {
                        tx.send(()).unwrap();
                    }
                    "longer" | "disabled" if label == "body:first" => {
                        tx.send(()).unwrap();
                    }
                    "concurrent" => {
                        started.push((label, tx));
                    }
                    _ => retained.push(tx),
                }
            }
        }
        if shared.mode == "concurrent" && started.len() == 2 {
            for timer in short_timers.drain(..) {
                timer.send(()).unwrap();
            }
            for (label, tx) in started.drain(..) {
                if label == "body:second" {
                    tx.send(()).unwrap();
                } else {
                    retained.push(tx);
                }
            }
        }
    }
}
async fn execute(world: &mut Acceptance) {
    let mode = world.mode.clone();
    let (tx, rx) = mpsc::unbounded();
    let shared = Arc::new(Shared {
        mode: mode.clone(),
        tx: tx.clone(),
        trace: Mutex::new(Vec::new()),
        handles: Mutex::new(Vec::new()),
        dropped: AtomicUsize::new(0),
        rejected: AtomicUsize::new(0),
        attempts: AtomicUsize::new(0),
        worlds: Mutex::new(Vec::new()),
        ticks: AtomicUsize::new(0),
    });
    let limits = Arc::new(Mutex::new(Vec::new()));
    let timeouts = if mode == "real" {
        Timeouts::default()
            .with_default(TimeoutPolicy::After(Duration::from_millis(1)))
    } else {
        let timeouts = match mode.as_str() {
            "legacy" => Timeouts::default(),
            "preset" => Timeouts::cucumber_js(),
            _ => Timeouts::default().with_default(TimeoutPolicy::After(SHORT)),
        };
        timeouts.with_clock(Arc::new(Clock {
            tx: tx.clone(),
            limits: limits.clone(),
        }))
    };
    let init = shared.clone();
    let cleanup = shared.clone();
    let before = shared.clone();
    let after = shared.clone();
    let scenario_hooks = Hooks::new()
        .with_store(world.scenario_hooks.clone())
        .before_with_timeout(
            "initialize",
            TimeoutPolicy::Disabled,
            move |_, _, scenario, w: &mut Application| {
                w.shared = Some(init.clone());
                w.name = scenario.name.clone();
                w.attempt = init.attempts.fetch_add(1, Ordering::SeqCst);
                init.worlds.lock().unwrap().push(w.id);
                Box::pin(async {})
            },
        )
        .before_with_timeout(
            "bounded setup",
            TimeoutPolicy::After(SHORT),
            move |_, _, _, w| {
                let shared = before.clone();
                Box::pin(async move {
                    shared.trace("before scenario");
                    if shared.mode == "before scenario" {
                        shared.retain(w);
                        gated(shared, "before scenario".into()).await;
                    }
                })
            },
        )
        .after_with_timeout(
            "final cleanup",
            TimeoutPolicy::Disabled,
            move |_, _, _, _, _| {
                let shared = cleanup.clone();
                Box::pin(async move {
                    shared.trace("final cleanup");
                    shared.reject_old();
                })
            },
        )
        .after_scoped_with_timeout(
            "bounded cleanup",
            TimeoutPolicy::After(SHORT),
            move |_, w| {
                let shared = after.clone();
                Box::pin(async move {
                    shared.trace("after scenario");
                    if shared.mode == "after scenario" {
                        shared.retain(w.unwrap());
                        gated(shared, "after scenario".into()).await;
                    }
                })
            },
        );
    let before = shared.clone();
    let after = shared.clone();
    let cleanup = shared.clone();
    let following = shared.clone();
    let step_hooks = StepHooks::new()
        .with_store(world.step_hooks.clone())
        .before_with_timeout(
            "bounded step setup",
            TimeoutPolicy::After(SHORT),
            move |_, w| {
                let shared = before.clone();
                Box::pin(async move {
                    shared.trace("before step");
                    if shared.mode == "before step" {
                        shared.retain(w);
                        gated(shared, "before step".into()).await;
                    }
                })
            },
        )
        .before_with_timeout(
            "following step setup",
            TimeoutPolicy::Disabled,
            move |_, _| {
                let shared = following.clone();
                Box::pin(async move {
                    if shared.mode == "before step" {
                        shared.reject_old();
                    }
                })
            },
        )
        .after_with_timeout(
            "step cleanup",
            TimeoutPolicy::Disabled,
            move |_, _, _| {
                let shared = cleanup.clone();
                Box::pin(async move {
                    shared.trace("step cleanup");
                    if shared.mode.ends_with("step") {
                        shared.reject_old();
                    }
                    if matches!(shared.mode.as_str(), "cancel" | "retry") {
                        shared.reject_old();
                        assert_eq!(shared.dropped.load(Ordering::SeqCst), 1);
                    }
                })
            },
        )
        .after_with_timeout(
            "bounded step cleanup",
            TimeoutPolicy::After(SHORT),
            move |_, _, w| {
                let shared = after.clone();
                Box::pin(async move {
                    shared.trace("after step");
                    if shared.mode == "after step" {
                        shared.retain(w);
                        gated(shared, "after step".into()).await;
                    }
                })
            },
        );
    let before = shared.clone();
    let after = shared.clone();
    let cleanup = shared.clone();
    let run_hooks = RunHooks::new()
        .with_store(world.run_hooks.clone())
        .before_with_timeout(
            "bounded run setup",
            TimeoutPolicy::After(SHORT),
            move |_| {
                let shared = before.clone();
                Box::pin(async move {
                    shared.trace("before run");
                    if shared.mode == "before run" {
                        gated(shared, "before run".into()).await;
                    }
                })
            },
        )
        .after_with_timeout("run cleanup", TimeoutPolicy::Disabled, move |_| {
            let shared = cleanup.clone();
            Box::pin(async move {
                shared.trace("run cleanup");
            })
        })
        .after_with_timeout(
            "bounded run cleanup",
            TimeoutPolicy::After(SHORT),
            move |_| {
                let shared = after.clone();
                Box::pin(async move {
                    shared.trace("after run");
                    if shared.mode == "after run" {
                        gated(shared, "after run".into()).await;
                    }
                })
            },
        );
    // Non-target hooks explicitly disable their own limits: body limit
    // observations
    // then have a precise independent meaning and legacy applications create
    // none.
    let hooks_only = matches!(
        mode.as_str(),
        "before scenario"
            | "after scenario"
            | "before step"
            | "after step"
            | "before run"
            | "after run"
    );
    let basic_hooks = Hooks::new()
        .before_with_timeout("initialize", TimeoutPolicy::Disabled, {
            let init = shared.clone();
            move |_, _, scenario, w: &mut Application| {
                w.shared = Some(init.clone());
                w.name = scenario.name.clone();
                w.attempt = init.attempts.fetch_add(1, Ordering::SeqCst);
                init.worlds.lock().unwrap().push(w.id);
                Box::pin(async {})
            }
        })
        .after_with_timeout("final cleanup", TimeoutPolicy::Disabled, {
            let cleanup = shared.clone();
            move |_, _, _, _, _| {
                let shared = cleanup.clone();
                Box::pin(async move {
                    shared.trace("final cleanup");
                    shared.reject_old();
                })
            }
        });
    let basic_step_hooks = StepHooks::new().after_with_timeout(
        "step cleanup",
        TimeoutPolicy::Disabled,
        {
            let cleanup = shared.clone();
            move |_, _, _| {
                let shared = cleanup.clone();
                Box::pin(async move {
                    shared.trace("step cleanup");
                    if shared.mode == "cancel"
                        || (shared.mode == "retry"
                            && shared.attempts.load(Ordering::SeqCst) == 1)
                    {
                        shared.reject_old();
                        assert_eq!(shared.dropped.load(Ordering::SeqCst), 1);
                    }
                })
            }
        },
    );
    let steps =
        Collection::new()
            .given_with_options(
                None,
                Regex::new("^(action|first)$").unwrap(),
                body,
                match mode.as_str() {
                    "longer" => DefinitionOptions::new()
                        .timeout(TimeoutPolicy::After(LONG)),
                    "disabled" => DefinitionOptions::new()
                        .timeout(TimeoutPolicy::Disabled),
                    _ => DefinitionOptions::new(),
                },
            )
            .given_with_options(
                None,
                Regex::new("^second$").unwrap(),
                body,
                if mode == "concurrent" {
                    DefinitionOptions::new().timeout(TimeoutPolicy::After(LONG))
                } else {
                    DefinitionOptions::new()
                },
            );
    let source = match mode.as_str() {
        "concurrent" => {
            "Feature: Deadlines\n Scenario: First\n  Given first\n Scenario: \
                Second\n  Given second\n"
        }
        "longer" | "disabled" => {
            "Feature: Deadlines\n Scenario: Work\n  Given first\n  Given \
                second\n"
        }
        _ => "Feature: Deadlines\n Scenario: Work\n  Given action\n",
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("timeouts.feature");
    std::fs::write(&path, source).unwrap();
    let sources = cucumber::parser::SourceStore::new();
    let attachments = AttachmentStore::new();
    let mut output = Vec::new();
    {
        let writer = cucumber::writer::Messages::raw_with_source_store(
            &mut output,
            sources.clone(),
        )
        .with_attachment_store(attachments.clone())
        .with_run_hook_store(world.run_hooks.clone())
        .with_scenario_hook_store(world.scenario_hooks.clone())
        .normalized()
        .discard_arbitrary_writes()
        .summarized();
        let run = cucumber::Cucumber::new()
            .steps(steps)
            .with_timeouts(timeouts)
            .with_source_store(sources)
            .with_attachments(attachments)
            .with_hooks(if hooks_only { scenario_hooks } else { basic_hooks })
            .with_step_hooks(if hooks_only {
                step_hooks
            } else {
                basic_step_hooks
            })
            .max_concurrent_scenarios(2)
            .retries(usize::from(mode == "retry"))
            .with_writer(writer)
            .with_run_hooks(if hooks_only {
                run_hooks
            } else {
                RunHooks::new()
            })
            .with_default_cli()
            .run(&path);
        let runner = async {
            let report = run.await;
            tx.unbounded_send(Signal::Done).unwrap();
            report
        };
        let driven = futures::future::join(runner, drive(rx, shared.clone()));
        futures::pin_mut!(driven);
        let safety = ThreadTimer.sleep(Duration::from_secs(15));
        let (report, ()) = match futures::future::select(driven, safety).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => panic!("deadline acceptance deadlocked"),
        };
        assert_eq!(report.parsing_errors(), 0);
        world.failed = report.failed_steps();
        world.passed = report.passed_steps();
        world.skipped = report.skipped_steps();
        world.hooks_failed = report.hook_errors();
        world.failed_run = report.execution_has_failed();
        drop(report);
    }
    world.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let schema: Value = serde_json::from_str(include_str!(
        "fixtures/messages/messages-34.2.1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for envelope in &world.messages {
        let errors: Vec<_> = validator
            .iter_errors(envelope)
            .map(|error| error.to_string())
            .collect();
        assert!(errors.is_empty(), "{envelope}: {errors:?}");
    }
    world.limits = limits.lock().unwrap().clone();
    world.shared = Some(shared);
}
fn shared(world: &Acceptance) -> &Arc<Shared> {
    world.shared.as_ref().unwrap()
}
fn all<'a>(world: &'a Acceptance, kind: &str) -> Vec<&'a Value> {
    world.messages.iter().filter_map(|value| value.get(kind)).collect()
}
fn statuses(world: &Acceptance) -> Vec<&str> {
    all(world, "testStepFinished")
        .into_iter()
        .map(|value| value["testStepResult"]["status"].as_str().unwrap())
        .collect()
}
fn timeout_messages(world: &Acceptance) -> Vec<&Value> {
    all(world, "testStepFinished")
        .into_iter()
        .filter(|value| {
            value["testStepResult"]["message"]
                .as_str()
                .is_some_and(|s| s.contains("timed out"))
        })
        .collect()
}
#[given("an application using the legacy timeout defaults")]
fn legacy(w: &mut Acceptance) {
    w.mode = "legacy".into();
}
#[given("an application using the JavaScript timeout preset")]
fn preset(w: &mut Acceptance) {
    w.mode = "preset".into();
}
#[given("an application whose body remains pending with scoped attachments")]
fn cancel(w: &mut Acceptance) {
    w.mode = "cancel".into();
}
#[given("an application whose first body attempt exceeds its deadline")]
fn retry(w: &mut Acceptance) {
    w.mode = "retry".into();
}
#[given(expr = "two application steps with the first override {string}")]
fn overrides(w: &mut Acceptance, mode: String) {
    w.mode = mode;
}
#[given(expr = "an application whose {string} callback remains pending")]
fn hook(w: &mut Acceptance, stage: String) {
    w.mode = stage;
}
#[given("two overlapping scenarios with independent callback deadlines")]
fn concurrent(w: &mut Acceptance) {
    w.mode = "concurrent".into();
}
#[given("an application on a plain futures executor")]
fn plain(w: &mut Acceptance) {
    assert!(tokio::runtime::Handle::try_current().is_err());
    w.mode = "real".into();
}
#[when(
    "its gated step is released after an injected deadline would have elapsed"
)]
#[when("it invokes a step with no definition override")]
#[when("its callback deadline expires")]
#[when("it runs with one retry and a completing second attempt")]
#[when("their inherited and overridden deadlines are exercised")]
#[when("that callback deadline expires")]
#[when("only one deadline expires and the other body completes")]
#[when("a real timer expires while its body remains pending")]
async fn run(w: &mut Acceptance) {
    execute(w).await;
}
#[then("the step passes without a timeout")]
fn legacy_pass(w: &mut Acceptance) {
    assert_eq!((w.failed, w.passed, w.hooks_failed), (0, 1, 0));
    assert!(w.limits.is_empty());
    assert!(shared(w).ticks.load(Ordering::SeqCst) > 0);
    assert!(!w.failed_run);
}
#[then("the callback receives a five-second deadline")]
fn five_seconds(w: &mut Acceptance) {
    assert_eq!(w.limits, [Duration::from_secs(5)]);
    assert_eq!((w.failed, w.passed), (0, 1));
}
#[then("its body is dropped and reports a timeout failure")]
fn cancelled(w: &mut Acceptance) {
    assert_eq!(shared(w).dropped.load(Ordering::SeqCst), 1);
    assert_eq!((w.failed, w.hooks_failed), (1, 0));
    assert_eq!(timeout_messages(w).len(), 1);
    assert!(w.failed_run);
    assert_eq!(all(w, "attachment").len(), 1);
}
#[then("its retained attachment handle rejects a later write")]
fn closed(w: &mut Acceptance) {
    assert!(shared(w).rejected.load(Ordering::SeqCst) >= 2);
    shared(w).reject_old();
}
#[then("cleanup still executes")]
fn cleanup(w: &mut Acceptance) {
    let trace = shared(w).trace.lock().unwrap();
    assert!(trace.iter().any(|s| s == "step cleanup"));
    assert!(trace.iter().any(|s| s == "final cleanup"));
}
#[then("both attempts retain distinct failure and success evidence")]
fn retries(w: &mut Acceptance) {
    assert_eq!((w.failed, w.passed, w.hooks_failed), (0, 1, 0));
    assert!(!w.failed_run);
    assert_eq!(timeout_messages(w).len(), 1);
    let attempts = all(w, "testCaseStarted");
    assert_eq!(attempts.len(), 2);
    assert_ne!(attempts[0]["id"], attempts[1]["id"]);
    assert_eq!(attempts[0]["attempt"], 0);
    assert_eq!(attempts[1]["attempt"], 1);
    let evidence = all(w, "attachment");
    assert_eq!(evidence.len(), 2);
    assert_eq!(evidence[0]["body"], "attempt:0");
    assert_eq!(evidence[1]["body"], "attempt:1");
    assert_ne!(
        evidence[0]["testCaseStartedId"],
        evidence[1]["testCaseStartedId"]
    );
    let worlds = shared(w).worlds.lock().unwrap();
    assert_eq!(worlds.len(), 2);
    assert_ne!(worlds[0], worlds[1]);
}
#[then(expr = "only the first step uses {string} and the second keeps the \
        default")]
fn local(w: &mut Acceptance, mode: String) {
    assert_eq!(
        w.limits,
        if mode == "longer" { vec![LONG, SHORT] } else { vec![SHORT] }
    );
    assert_eq!((w.passed, w.failed), (1, 1));
    assert_eq!(timeout_messages(w).len(), 1);
}
#[then("its timeout is reported at the actual hook or containing step")]
fn hook_failure(w: &mut Acceptance) {
    assert!(w.failed_run);
    assert_eq!(shared(w).dropped.load(Ordering::SeqCst), 1);
    if !w.mode.ends_with("run") {
        assert!(shared(w).rejected.load(Ordering::SeqCst) > 0);
        assert_eq!(all(w, "attachment").len(), 1);
    }
    if w.mode.ends_with("run") {
        let failed: Vec<_> = w
            .run_hooks
            .records_for(w.run_hooks.current_run().unwrap())
            .into_iter()
            .filter(|r| r.result.is_err())
            .collect();
        assert_eq!(failed.len(), 1);
        assert!(failed[0].result.as_ref().unwrap_err().contains("timed out"));
        assert_eq!(
            all(w, "testRunHookFinished")
                .into_iter()
                .filter(|value| value["result"]["status"] == "FAILED")
                .count(),
            1
        );
        assert_eq!(w.hooks_failed, 1);
    } else if w.mode.ends_with("scenario") {
        let failed: Vec<_> = w
            .scenario_hooks
            .records()
            .into_iter()
            .filter(|r| {
                matches!(
                    &r.disposition,
                    ScenarioHookDisposition::Executed { result: Err(_), .. }
                )
            })
            .collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(w.hooks_failed, 1);
        assert_eq!(timeout_messages(w).len(), 1);
    } else {
        let failed: Vec<_> = w
            .step_hooks
            .records()
            .into_iter()
            .filter(|r| r.result.is_err())
            .collect();
        assert_eq!(failed.len(), 1);
        assert!(failed[0].result.as_ref().unwrap_err().contains("timed out"));
        assert_eq!((w.failed, w.hooks_failed), (1, 0));
        assert_eq!(timeout_messages(w).len(), 1);
    }
}
#[then("the remaining cleanup follows the established lifecycle policy")]
fn hook_cleanup(w: &mut Acceptance) {
    let trace = shared(w).trace.lock().unwrap();
    assert!(trace.iter().any(|s| s == "run cleanup"));
    if w.mode != "before run" {
        assert!(trace.iter().any(|s| s == "final cleanup"));
    }
    if w.mode == "before step" || w.mode == "after step" {
        assert!(trace.iter().any(|s| s == "step cleanup"));
    }
    if w.mode == "before run"
        || w.mode == "before scenario"
        || w.mode == "before step"
    {
        assert!(!trace.iter().any(|s| s.starts_with("body:")));
    }
}
#[then("one scenario fails and the other passes without crossed attribution")]
fn isolated(w: &mut Acceptance) {
    assert_eq!((w.failed, w.passed, w.hooks_failed), (1, 1, 0));
    assert_eq!(shared(w).ticks.load(Ordering::SeqCst), 2);
    let failed = timeout_messages(w);
    assert_eq!(failed.len(), 1);
    let id = &failed[0]["testCaseStartedId"];
    let case =
        all(w, "testCaseStarted").into_iter().find(|c| &c["id"] == id).unwrap();
    let plan = all(w, "testCase")
        .into_iter()
        .find(|c| c["id"] == case["testCaseId"])
        .unwrap();
    let pickle = all(w, "pickle")
        .into_iter()
        .find(|p| p["id"] == plan["pickleId"])
        .unwrap();
    assert_eq!(pickle["name"], "First");
    assert!(statuses(w).contains(&"PASSED"));
}
#[then("the timeout is reported without requiring a runtime-specific reactor")]
fn independent(w: &mut Acceptance) {
    assert!(tokio::runtime::Handle::try_current().is_err());
    assert_eq!((w.failed, w.hooks_failed), (1, 0));
    assert_eq!(timeout_messages(w).len(), 1);
    assert_eq!(shared(w).dropped.load(Ordering::SeqCst), 1);
}
fn main() {
    futures::executor::block_on(
        Acceptance::cucumber()
            .max_concurrent_scenarios(1)
            .fail_on_skipped()
            .run_and_exit("tests/features/timeouts"),
    );
}
