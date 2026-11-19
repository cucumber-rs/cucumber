//! Phase30b: scoped completion through the real native runner and lifecycle
//! registries.
use std::{
    collections::HashSet,
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use cucumber::{
    Event, StatsWriter as _, World as _, WriterExt as _, event,
    execution::{
        Completion, ControlError, ExecutionControl, ExecutionHandle, Outcome,
        OutcomePolicy, OutcomeStatus, WorldContext,
    },
    given,
    runner::{
        Hooks, ScenarioHookDisposition, ScenarioHookScope, ScenarioHookStore,
        StepHookStore, StepHooks, TimeoutPolicy,
    },
    step::{Collection, Context, DefinitionOptions, Matching},
    then, when,
    writer::{self, Coloring},
};
use futures::future::LocalBoxFuture;
use regex::Regex;
#[derive(Debug, Default)]
struct Seen {
    trace: Vec<(String, usize, String)>,
    handles: Vec<ExecutionHandle>,
    worlds: Vec<usize>,
    scopes: Vec<ScenarioHookScope>,
    second_rejections: usize,
}
#[derive(Debug, Default)]
struct Application {
    control: ExecutionControl,
    seen: Arc<Mutex<Seen>>,
    mode: String,
    name: String,
    attempt: usize,
    id: usize,
    previous: Option<ExecutionHandle>,
    barrier: Option<Arc<tokio::sync::Barrier>>,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Ok(Self { id: NEXT.fetch_add(1, Ordering::SeqCst), ..Self::default() })
    }
    async fn new_with_runtime_context(
        context: WorldContext,
    ) -> Result<Self, Self::Error> {
        assert_eq!(
            context.execution().current().unwrap_err(),
            ControlError::NoActiveScope
        );
        let mut world = Self::new().await?;
        world.control = context.execution();
        Ok(world)
    }
}
fn enter(world: &mut Application, label: &str) {
    if let Some(previous) = world.previous.take() {
        assert_eq!(
            previous.complete(Completion::Skipped),
            Err(ControlError::ClosedScope)
        );
    }
    world.seen.lock().unwrap().trace.push((
        world.name.clone(),
        world.attempt,
        label.into(),
    ));
}
fn request(world: &mut Application, completion: Completion) {
    let handle = world.control.current().unwrap();
    handle.complete(completion).unwrap();
    assert_eq!(
        handle.complete(Completion::Pending),
        Err(ControlError::AlreadyCompleted)
    );
    let mut seen = world.seen.lock().unwrap();
    seen.second_rejections += 1;
    seen.handles.push(handle.clone());
    world.previous = Some(handle);
}
fn configured_request(world: &mut Application, location: &str) {
    if world.mode == format!("{location} pending") {
        request(world, Completion::Pending);
    }
    if world.mode == format!("{location} skipped") {
        request(world, Completion::Skipped);
    }
}
fn body(world: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        enter(world, "body");
        configured_request(world, "step");
        match world.mode.as_str() {
            "explicit skip then unresolved" => {
                request(world, Completion::Skipped)
            }
            "pending then unresolved" | "pending then cleanup failure" => {
                request(world, Completion::Pending)
            }
            "failure then unresolved" => panic!("actual body failure"),
            "concurrent scoped handles" => {
                world.barrier.as_ref().unwrap().wait().await;
                let completion = if world.name == "left" {
                    Completion::Pending
                } else {
                    Completion::Skipped
                };
                request(world, completion);
            }
            "pending then panic or timeout" => {
                request(world, Completion::Pending);
                if world.name == "panic" {
                    panic!("actual failure after pending");
                }
                futures::future::pending::<()>().await;
            }
            _ => {}
        }
    })
}
#[derive(Debug, Clone)]
struct Row {
    name: String,
    attempt: usize,
    kind: String,
    outcome: Option<Outcome>,
    projection: &'static str,
}
#[derive(Debug, Clone, Default)]
struct Recorder(Arc<Mutex<Vec<Row>>>);
impl writer::NonTransforming for Recorder {}
impl cucumber::Writer<Application> for Recorder {
    type Cli = cucumber::cli::Empty;
    async fn handle_event(
        &mut self,
        input: cucumber::parser::Result<Event<event::Cucumber<Application>>>,
        _: &Self::Cli,
    ) {
        let (event, meta) = input.unwrap().split();
        let (scenario, event) = match event {
            event::Cucumber::Feature(_, event::Feature::Scenario(s, e)) => {
                (s, e)
            }
            event::Cucumber::Feature(
                _,
                event::Feature::Rule(_, event::Rule::Scenario(s, e)),
            ) => (s, e),
            _ => return,
        };
        // Deliberately exhaustive old native variants: source compatibility is
        // part of this contract.
        let (kind, projection) = match event.event {
            event::Scenario::Step(step, result)
            | event::Scenario::Background(step, result) => {
                let projection = match result {
                    event::Step::Started => "started",
                    event::Step::Skipped => "skipped",
                    event::Step::Passed(..) => "passed",
                    event::Step::Failed(..) => "failed",
                };
                (step.value.clone(), projection)
            }
            event::Scenario::Hook(kind, result) => {
                let projection = match result {
                    event::Hook::Started => "started",
                    event::Hook::Passed => "passed",
                    event::Hook::Failed(..) => "failed",
                };
                (format!("{kind:?}"), projection)
            }
            event::Scenario::Finished => ("finished".into(), "finished"),
            _ => return,
        };
        self.0.lock().unwrap().push(Row {
            name: scenario.name.clone(),
            attempt: event.retries.map_or(0, |r| r.current),
            kind,
            outcome: meta.outcome,
            projection,
        });
    }
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    strict: bool,
    seen: Arc<Mutex<Seen>>,
    scenario_store: ScenarioHookStore,
    step_store: StepHookStore,
    recorder: Recorder,
    failed: bool,
    output: String,
    constructors: bool,
}
fn before(
    name: &'static str,
    seen: Arc<Mutex<Seen>>,
    mode: String,
    barrier: Option<Arc<tokio::sync::Barrier>>,
) -> impl for<'a> Fn(
    &'a ScenarioHookScope,
    &'a mut Application,
) -> LocalBoxFuture<'a, ()> {
    move |scope, world| {
        let seen = seen.clone();
        let mode = mode.clone();
        let barrier = barrier.clone();
        Box::pin(async move {
            world.seen = seen;
            world.mode = mode;
            world.name = scope.attempt.scenario.name.clone();
            world.attempt = scope.attempt.attempt;
            world.barrier = barrier;
            if name == "before-one" {
                world.seen.lock().unwrap().worlds.push(world.id);
            }
            enter(world, name);
            if name == "before-one" {
                configured_request(world, "before scenario");
            }
        })
    }
}
fn after(
    name: &'static str,
) -> impl for<'a> Fn(
    &'a ScenarioHookScope,
    Option<&'a mut Application>,
) -> LocalBoxFuture<'a, ()> {
    move |scope, world| {
        Box::pin(async move {
            let world = world.expect("these applications initialize World");
            enter(world, name);
            world.seen.lock().unwrap().scopes.push(scope.clone());
            if name == "after-two" {
                configured_request(world, "after scenario");
                assert!(
                    !(world.mode == "pending then cleanup failure"
                        && world.attempt == 0),
                    "actual cleanup failure"
                );
            }
        })
    }
}
#[given(expr = "execution control {string} with strict pending {string}")]
fn configure(w: &mut Acceptance, mode: String, strict: String) {
    w.mode = mode;
    w.strict = strict.parse().unwrap();
}
#[when("the application runs with the explicit JS outcome policy and retries")]
async fn run(w: &mut Acceptance) {
    let barrier = (w.mode == "concurrent scoped handles")
        .then(|| Arc::new(tokio::sync::Barrier::new(2)));
    let hooks = Hooks::new()
        .with_store(w.scenario_store.clone())
        .before_scoped(
            "before-one",
            before(
                "before-one",
                w.seen.clone(),
                w.mode.clone(),
                barrier.clone(),
            ),
        )
        .before_scoped(
            "before-two",
            before("before-two", w.seen.clone(), w.mode.clone(), barrier),
        )
        .after_scoped("after-one", after("after-one"))
        .after_scoped("after-two", after("after-two"));
    let step_hooks = StepHooks::new()
        .with_store(w.step_store.clone())
        .before("step-before-one", |_, world: &mut Application| {
            Box::pin(async move {
                enter(world, "step-before-one");
                configured_request(world, "before step");
                if world.mode == "step hooks control" {
                    request(world, Completion::Pending);
                }
            })
        })
        .before("step-before-two", |_, world| {
            Box::pin(async move {
                enter(world, "step-before-two");
                if world.mode == "step hooks control" {
                    request(world, Completion::Skipped);
                }
            })
        })
        .after("step-after-one", |_, _, world| {
            Box::pin(async move {
                enter(world, "step-after-one");
            })
        })
        .after("step-after-two", |_, _, world| {
            Box::pin(async move {
                enter(world, "step-after-two");
                configured_request(world, "after step");
                if world.mode == "step hooks control" {
                    request(world, Completion::Skipped);
                }
            })
        });
    let names = if w.mode == "concurrent scoped handles" {
        vec!["left", "right"]
    } else if w.mode == "pending then panic or timeout" {
        vec!["panic", "timeout"]
    } else {
        vec!["application"]
    };
    let mut source = "Feature: Scoped completion\n".to_owned();
    for name in names {
        source.push_str(&format!(" Scenario: {name}\n  Given work\n"));
        if w.mode.ends_with("then unresolved") {
            source.push_str("  Given undefined\n  Given ambiguous\n");
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("control.feature");
    std::fs::write(&path, source).unwrap();
    let timeout = if w.mode == "pending then panic or timeout" {
        TimeoutPolicy::After(Duration::from_millis(10))
    } else {
        TimeoutPolicy::Disabled
    };
    let steps = Collection::new()
        .given_with_options(
            None,
            Regex::new("^work$").unwrap(),
            body,
            DefinitionOptions::new().timeout(timeout),
        )
        .given(None, Regex::new("^ambiguous$").unwrap(), body)
        .when(None, Regex::new("^ambiguous$").unwrap(), body);
    let mut output = Vec::new();
    let writer = writer::Basic::raw(&mut output, Coloring::Never, 0)
        .tee::<Application, _>(w.recorder.clone().discard_arbitrary_writes())
        .normalized()
        .summarized();
    let report = cucumber::Cucumber::new()
        .steps(steps)
        .step_matching(Matching::KeywordIndependent)
        .with_hooks(hooks)
        .with_step_hooks(step_hooks)
        .outcome_policy(OutcomePolicy::CucumberJs { strict_pending: w.strict })
        .max_concurrent_scenarios(2)
        .retries(1)
        .with_writer(writer)
        .with_default_cli()
        .run(path)
        .await;
    assert_eq!(report.parsing_errors(), 0);
    w.failed = report.execution_has_failed();
    drop(report);
    w.output = String::from_utf8(output).unwrap();
}
fn finals(w: &Acceptance) -> Vec<Row> {
    w.recorder
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|row| row.kind == "finished")
        .cloned()
        .collect()
}
fn status(w: &Acceptance) -> OutcomeStatus {
    finals(w).last().unwrap().outcome.unwrap().status
}
fn assert_closed(w: &Acceptance) {
    let seen = w.seen.lock().unwrap();
    assert!(!seen.handles.is_empty());
    for handle in &seen.handles {
        assert_eq!(
            handle.complete(Completion::Skipped),
            Err(ControlError::ClosedScope)
        );
    }
}
#[then("the report contains a PENDING outcome and no invented panic")]
fn pending(w: &mut Acceptance) {
    assert_eq!(status(w), OutcomeStatus::Pending, "{}", w.output);
    assert!(w.step_store.records().iter().all(|r| r.result.is_ok()));
    assert!(w.scenario_store.records().iter().all(|r| match &r.disposition {
        ScenarioHookDisposition::Executed { result, .. } => result.is_ok(),
        _ => true,
    }));
    assert!(!w.output.contains("panicked"), "{}", w.output);
    assert_closed(w);
}
#[then(expr = "pending completes one attempt with final success {string}")]
fn pending_success(w: &mut Acceptance, success: String) {
    let rows = finals(w);
    assert_eq!(rows.len(), 1);
    assert_eq!(w.failed, !success.parse::<bool>().unwrap());
    assert!(!rows[0].outcome.unwrap().retryable);
}
#[then("all applicable cleanup completes")]
fn cleanup(w: &mut Acceptance) {
    let seen = w.seen.lock().unwrap();
    assert_eq!(
        seen.trace.iter().filter(|(_, _, label)| label == "after-one").count(),
        1
    );
    assert_eq!(
        seen.trace.iter().filter(|(_, _, label)| label == "after-two").count(),
        1
    );
    if !w.mode.starts_with("before scenario") {
        for label in ["step-after-one", "step-after-two"] {
            assert_eq!(
                seen.trace.iter().filter(|(_, _, l)| l == label).count(),
                1
            );
        }
    } else {
        assert!(
            !seen
                .trace
                .iter()
                .any(|(_, _, l)| l == "body" || l == "before-two")
        );
    }
}
#[then("the report contains deliberate SKIPPED without UNDEFINED")]
fn skipped(w: &mut Acceptance) {
    assert_eq!(status(w), OutcomeStatus::Skipped, "{}", w.output);
    assert!(w.recorder.0.lock().unwrap().iter().all(|r| {
        r.outcome.is_none_or(|o| o.status != OutcomeStatus::Undefined)
    }));
    assert_closed(w);
}
#[then("deliberate skip completes one successful attempt")]
fn skip_success(w: &mut Acceptance) {
    assert_eq!(finals(w).len(), 1);
    assert!(!w.failed, "{}", w.output);
    assert!(!finals(w)[0].outcome.unwrap().retryable);
}
#[then("every selected step hook and the resolved body execute")]
fn step_trace(w: &mut Acceptance) {
    let seen = w.seen.lock().unwrap();
    let actual: Vec<_> =
        seen.trace.iter().map(|(_, _, label)| label.as_str()).collect();
    assert_eq!(
        actual,
        vec![
            "before-one",
            "before-two",
            "step-before-one",
            "step-before-two",
            "body",
            "step-after-two",
            "step-after-one",
            "after-two",
            "after-one"
        ]
    );
    assert_eq!(w.step_store.records().len(), 4);
}
#[then("the actual aggregate uses the most severe outcome")]
fn worst(w: &mut Acceptance) {
    assert_eq!(status(w), OutcomeStatus::Pending);
    assert!(w.failed);
    assert_eq!(finals(w).len(), 1);
}
#[then(expr = "later step statuses are {string}")]
fn lookup(w: &mut Acceptance, expected: String) {
    let rows = w.recorder.0.lock().unwrap();
    let names = ["work", "undefined", "ambiguous"];
    let actual: Vec<_> = rows
        .iter()
        .filter(|r| {
            r.attempt == 0
                && names.contains(&r.kind.as_str())
                && r.projection != "started"
        })
        .map(|r| format!("{:?}", r.outcome.unwrap().status).to_uppercase())
        .collect();
    assert_eq!(actual.join(","), expected, "{}", w.output);
    let ordered: Vec<_> = rows
        .iter()
        .filter(|r| r.attempt == 0 && names.contains(&r.kind.as_str()))
        .map(|r| (r.kind.as_str(), r.projection == "started"))
        .collect();
    assert_eq!(
        ordered,
        vec![
            ("work", true),
            ("work", false),
            ("undefined", true),
            ("undefined", false),
            ("ambiguous", true),
            ("ambiguous", false)
        ]
    );
    assert_eq!(
        w.seen
            .lock()
            .unwrap()
            .trace
            .iter()
            .filter(|(_, a, l)| *a == 0 && l == "body")
            .count(),
        1
    );
}
#[then("only the actual failure enables a retry with a fresh World")]
fn retry(w: &mut Acceptance) {
    let rows = finals(w);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].outcome.unwrap().status, OutcomeStatus::Failed);
    assert!(rows[0].outcome.unwrap().retryable);
    assert_eq!(rows[1].attempt, 1);
    let seen = w.seen.lock().unwrap();
    assert_eq!(seen.worlds.len(), 2);
    assert_ne!(seen.worlds[0], seen.worlds[1]);
    assert_eq!(
        seen.trace.iter().filter(|(_, _, l)| l == "after-one").count(),
        2
    );
}
#[then("the final pending attempt retains its strict failure outcome")]
fn retry_pending(w: &mut Acceptance) {
    assert_eq!(status(w), OutcomeStatus::Pending);
    assert!(w.failed);
    assert_closed(w);
    let seen = w.seen.lock().unwrap();
    let first = seen
        .scopes
        .iter()
        .find(|s| s.attempt.attempt == 0 && s.definition.name == "after-one")
        .unwrap();
    assert_eq!(first.outcome.unwrap().status, OutcomeStatus::Failed);
    let second = seen
        .scopes
        .iter()
        .find(|s| s.attempt.attempt == 1 && s.definition.name == "after-one")
        .unwrap();
    assert_eq!(second.outcome.unwrap().status, OutcomeStatus::Pending);
    assert!(second.result.as_ref().unwrap().previous_hook_failures.is_empty());
}
#[then(
    "closed handles reject requests and concurrent cases retain their own \
        outcome"
)]
fn concurrency(w: &mut Acceptance) {
    assert_closed(w);
    let rows = finals(w);
    assert_eq!(rows.len(), 2);
    for (name, expected) in
        [("left", OutcomeStatus::Pending), ("right", OutcomeStatus::Skipped)]
    {
        assert_eq!(
            rows.iter()
                .find(|r| r.name == name)
                .unwrap()
                .outcome
                .unwrap()
                .status,
            expected
        );
    }
    assert!(!w.failed);
    let ids: HashSet<_> =
        w.seen.lock().unwrap().worlds.iter().copied().collect();
    assert_eq!(ids.len(), 2);
}
#[then("a second request cannot replace the first completion decision")]
fn second(w: &mut Acceptance) {
    let seen = w.seen.lock().unwrap();
    assert_eq!(seen.second_rejections, 2);
    assert_eq!(seen.handles.len(), 2);
}
#[then("the actual panic or timeout remains FAILED")]
fn failure(w: &mut Acceptance) {
    let rows = finals(w);
    assert_eq!(rows.len(), 4);
    assert!(
        rows.iter().all(|r| r.outcome.unwrap().status == OutcomeStatus::Failed)
    );
    assert!(w.failed);
    assert!(w.output.contains("actual failure after pending"));
    assert!(w.output.contains("timed out"));
}
#[then("expiry closes the completion scope")]
fn expired(w: &mut Acceptance) {
    assert_closed(w);
    assert_eq!(
        w.seen
            .lock()
            .unwrap()
            .trace
            .iter()
            .filter(|(_, _, l)| l == "after-one")
            .count(),
        4
    );
}
#[given("a custom writer using the existing native event variants")]
fn custom(w: &mut Acceptance) {
    w.mode = "step pending".into();
    w.strict = true;
}
#[when("an explicitly controlled application runs through that writer")]
async fn custom_run(w: &mut Acceptance) {
    run(w).await;
}
#[then(
    "the custom writer compiles and sees a documented non-passing projection"
)]
fn projection(w: &mut Acceptance) {
    let rows = w.recorder.0.lock().unwrap();
    let row = rows
        .iter()
        .find(|r| r.kind == "work" && r.projection != "started")
        .unwrap();
    assert_eq!(row.projection, "skipped");
}
#[then("metadata-aware consumers retain the exact completion status")]
fn metadata(w: &mut Acceptance) {
    assert_eq!(status(w), OutcomeStatus::Pending);
    assert!(w.failed);
}
#[derive(Debug, Default)]
struct Original {
    initialized: bool,
}
impl cucumber::World for Original {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self { initialized: true })
    }
}
fn original_step(w: &mut Original, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        assert!(w.initialized);
    })
}
#[derive(Debug, Default)]
struct AttachmentAware {
    context: cucumber::attachment::AttachmentContext,
    initialized: bool,
}
impl cucumber::World for AttachmentAware {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
    async fn new_with_context(
        context: cucumber::attachment::AttachmentContext,
    ) -> Result<Self, Self::Error> {
        context
            .scope()
            .unwrap()
            .attach(cucumber::attachment::Attachment::text(
                "constructor",
                "text/plain",
            ))
            .unwrap();
        Ok(Self { context, initialized: true })
    }
}
fn attachment_step(
    w: &mut AttachmentAware,
    _: Context,
) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        assert!(w.initialized);
        w.context
            .scope()
            .unwrap()
            .attach(cucumber::attachment::Attachment::text(
                "body",
                "text/plain",
            ))
            .unwrap();
    })
}
#[given(
    "original and attachment-aware World implementations with no \
        execution-control override"
)]
fn constructors(_: &mut Acceptance) {}
#[when("each application runs using its existing constructors")]
async fn original_constructors(w: &mut Acceptance) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.feature");
    std::fs::write(
        &path,
        "Feature: Original constructors\n Scenario: original\n  Given work\n",
    )
    .unwrap();
    let mut output = Vec::new();
    let report = cucumber::Cucumber::new()
        .steps(Collection::new().given(
            None,
            Regex::new("^work$").unwrap(),
            original_step,
        ))
        .outcome_policy(OutcomePolicy::cucumber_js())
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .run(&path)
        .await;
    assert_eq!(report.passed_steps(), 1);
    assert!(!report.execution_has_failed());
    drop(report);
    let store = cucumber::attachment::AttachmentStore::new();
    let report = cucumber::Cucumber::new()
        .steps(Collection::new().given(
            None,
            Regex::new("^work$").unwrap(),
            attachment_step,
        ))
        .with_attachments(store.clone())
        .outcome_policy(OutcomePolicy::cucumber_js())
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .run(path)
        .await;
    assert_eq!(report.passed_steps(), 1);
    assert!(!report.execution_has_failed());
    w.constructors = true;
}
#[then("their constructor behavior and attachment scope remain unchanged")]
fn constructors_unchanged(w: &mut Acceptance) {
    assert!(w.constructors);
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/execution_control")
        .await;
}
