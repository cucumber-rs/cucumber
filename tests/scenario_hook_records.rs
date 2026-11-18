//! Phase29 acceptance: real retained scenario registries and scoped observations.
use cucumber::{
    StatsWriter as _, World as _, WriterExt as _,
    attachment::{
        Attachment, AttachmentContext, AttachmentError, AttachmentHandle,
        AttachmentStore,
    },
    event::ScenarioFinished,
    given,
    runner::{
        EmptyScenarioHooks, Hooks, ScenarioHookDisposition, ScenarioHookKind,
        ScenarioHookScope, ScenarioHookSkipReason, ScenarioHookStatus,
        ScenarioHookStore,
    },
    step::{Collection, Context},
    then, when,
    writer::{self, Coloring},
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::{Arc, Mutex},
};
#[derive(Debug, Default)]
struct Seen {
    trace: Vec<String>,
    scopes: Vec<ScenarioHookScope>,
    legacy: Vec<ScenarioFinished>,
    bodies: usize,
    initial_sizes: Vec<usize>,
    handles: Vec<AttachmentHandle>,
    rejected: usize,
}
#[derive(Debug, Default)]
struct Application {
    context: AttachmentContext,
    seen: Arc<Mutex<Seen>>,
    mode: String,
    attempt: usize,
    local: Vec<String>,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
    async fn new_with_context(
        context: AttachmentContext,
    ) -> Result<Self, Self::Error> {
        if let Ok(handle) = context.scope() {
            handle
                .attach(Attachment::text("constructor", "text/plain"))
                .unwrap();
        }
        Ok(Self { context, ..Self::default() })
    }
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    store: ScenarioHookStore,
    seen: Arc<Mutex<Seen>>,
    hook_errors: usize,
    step_errors: usize,
    messages: Vec<Value>,
    output: Vec<u8>,
}
fn evidence(world: &Application, name: &str) {
    let handle = world.context.scope().unwrap();
    let mut seen = world.seen.lock().unwrap();
    if let Some(previous) = seen.handles.last() {
        assert_eq!(
            previous.attach(Attachment::text("late", "text/plain")),
            Err(AttachmentError::ClosedScope)
        );
        seen.rejected += 1;
    }
    handle.attach(Attachment::text(name, "text/plain")).unwrap();
    seen.handles.push(handle);
}
fn before(
    name: &'static str,
    seen: Arc<Mutex<Seen>>,
    mode: String,
) -> impl for<'a> Fn(
    &'a ScenarioHookScope,
    &'a mut Application,
) -> LocalBoxFuture<'a, ()> {
    move |scope, world| {
        let seen = seen.clone();
        let mode = mode.clone();
        if mode == "constructor panic" && name == "before-one" {
            seen.lock().unwrap().trace.push(name.into());
            panic!("callback construction failure");
        }
        Box::pin(async move {
            world.seen = seen.clone();
            world.mode = mode.clone();
            world.attempt = scope.attempt.attempt;
            {
                let mut seen = seen.lock().unwrap();
                seen.trace.push(name.into());
                seen.scopes.push(scope.clone());
                if name == "before-one" {
                    seen.initial_sizes.push(world.local.len());
                }
            }
            world.local.push(name.into());
            if matches!(mode.as_str(), "attachments" | "constructor evidence") {
                evidence(world, name);
            }
            assert!(
                !(name == "before-one"
                    && matches!(
                        mode.as_str(),
                        "setup failure" | "empty native" | "empty js failure"
                    )),
                "setup failed before-one"
            );
        })
    }
}
fn after(
    name: &'static str,
    seen: Arc<Mutex<Seen>>,
    mode: String,
) -> impl for<'a> Fn(
    &'a ScenarioHookScope,
    Option<&'a mut Application>,
) -> LocalBoxFuture<'a, ()> {
    move |scope, world| {
        let seen = seen.clone();
        let mode = mode.clone();
        Box::pin(async move {
            {
                let mut seen = seen.lock().unwrap();
                seen.trace.push(name.into());
                seen.scopes.push(scope.clone());
            }
            if matches!(mode.as_str(), "attachments" | "constructor evidence") {
                evidence(world.unwrap(), name);
            }
            assert!(
                !(name == "after-two"
                    && matches!(
                        mode.as_str(),
                        "cleanup failure"
                            | "body and cleanup failure"
                            | "mixed legacy and scoped"
                    )),
                "cleanup failed after-two"
            );
        })
    }
}
fn work(world: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        world.seen.lock().unwrap().bodies += 1;
        assert!(
            !(world.mode == "body and cleanup failure"
                || (world.mode == "retry" && world.attempt == 0)),
            "primary body failure"
        );
    })
}
fn registry(world: &Acceptance) -> Hooks<Application> {
    let mut hooks = Hooks::new().with_store(world.store.clone());
    if world.mode == "tags" || world.mode == "all excluded" {
        let expr = if world.mode == "tags" {
            "@feature and @rule and @scenario"
        } else {
            "@missing"
        };
        return hooks
            .before_scoped_matching(
                "before-one",
                expr.parse().unwrap(),
                before("before-one", world.seen.clone(), world.mode.clone()),
            )
            .before_scoped_matching(
                "excluded-before",
                "@absent".parse().unwrap(),
                before(
                    "excluded-before",
                    world.seen.clone(),
                    world.mode.clone(),
                ),
            )
            .after_scoped_matching(
                "after-one",
                expr.parse().unwrap(),
                after("after-one", world.seen.clone(), world.mode.clone()),
            )
            .after_scoped_matching(
                "excluded-after",
                "not @feature".parse().unwrap(),
                after("excluded-after", world.seen.clone(), world.mode.clone()),
            );
    }
    hooks = hooks
        .before_scoped(
            "before-one",
            before("before-one", world.seen.clone(), world.mode.clone()),
        )
        .before_scoped(
            "before-two",
            before("before-two", world.seen.clone(), world.mode.clone()),
        );
    if world.mode == "mixed legacy and scoped" {
        hooks = hooks.after_scoped(
            "after-one",
            after("after-one", world.seen.clone(), world.mode.clone()),
        );
        let seen = world.seen.clone();
        hooks = hooks.after("legacy-after", move |_, _, _, body, _| {
            let seen = seen.clone();
            Box::pin(async move {
                let mut s = seen.lock().unwrap();
                s.trace.push("legacy-after".into());
                s.legacy.push(body.clone());
            })
        });
    } else {
        hooks = hooks.after_scoped(
            "after-one",
            after("after-one", world.seen.clone(), world.mode.clone()),
        );
    }
    hooks = hooks.after_scoped(
        "after-two",
        after("after-two", world.seen.clone(), world.mode.clone()),
    );
    if world.mode.starts_with("empty js") {
        hooks = hooks.empty_scenario_policy(EmptyScenarioHooks::CucumberJs13_3);
    }
    hooks
}
#[given(expr = "an observed scenario-hook registry configured for {string}")]
fn configured(world: &mut Acceptance, mode: String) {
    world.mode = mode;
}
#[given(
    "an observed scenario-hook registry selected by inherited feature Rule and scenario tags"
)]
fn tags(w: &mut Acceptance) {
    w.mode = "tags".into();
}
#[given("two concurrent scenarios sharing an observed scenario-hook registry")]
fn concurrent(w: &mut Acceptance) {
    w.mode = "concurrent".into();
}
#[given(
    "an observed registry whose before stage is replaced using the legacy builder"
)]
fn replaced(w: &mut Acceptance) {
    w.mode = "replaced".into();
}
#[given(
    "an observed registry with failing setup and an empty scenario under native policy"
)]
fn empty_native(w: &mut Acceptance) {
    w.mode = "empty native".into();
}
#[given(
    "an observed registry with failing setup and an empty scenario under JS13.3 policy"
)]
fn empty_js(w: &mut Acceptance) {
    w.mode = "empty js failure".into();
}
#[given(
    "an observed registry with passing setup and an empty scenario under JS13.3 policy"
)]
fn empty_js_pass(w: &mut Acceptance) {
    w.mode = "empty js passing".into();
}
#[given("an unobserved legacy hook registry with typed attachments")]
fn unobserved(w: &mut Acceptance) {
    w.mode = "unobserved".into();
}
#[given("an observed registry whose World constructor fails")]
fn constructor_fails(w: &mut Acceptance) {
    w.mode = "world failure".into();
}
#[when("the application executes its scenario")]
#[when("the application executes both scenarios")]
async fn execute(world: &mut Acceptance) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("hooks.feature");
    let mut source =
        "@feature\nFeature: Observation\n  @rule\n  Rule: Request\n".to_owned();
    for name in if world.mode == "concurrent" {
        vec!["left", "right"]
    } else {
        vec!["one"]
    } {
        source.push_str(&format!("    @scenario\n    Scenario: {name}\n"));
        if !world.mode.starts_with("empty") {
            source.push_str("      Given work\n");
        }
    }
    std::fs::write(&path, source).unwrap();
    if world.mode == "world failure" {
        run_failed_constructor(world, &path).await;
        return;
    }
    let mut hooks = registry(world);
    let attachments = AttachmentStore::new();
    let sources = cucumber::parser::SourceStore::new();
    let use_messages = matches!(
        world.mode.as_str(),
        "attachments" | "constructor evidence" | "unobserved"
    );
    if world.mode == "unobserved" {
        hooks = Hooks::new();
        for name in ["before-one", "before-two"] {
            let seen = world.seen.clone();
            hooks = hooks.before(name, move |_, _, _, world| {
                let seen = seen.clone();
                Box::pin(async move {
                    world.seen = seen;
                    evidence(world, name);
                })
            });
        }
        for name in ["after-one", "after-two"] {
            hooks = hooks.after(name, move |_, _, _, _, world| {
                Box::pin(async move {
                    evidence(world.unwrap(), name);
                })
            });
        }
    }
    let mut output = Vec::new();
    if use_messages {
        let mut writer = writer::Messages::raw_with_source_store(
            &mut output,
            sources.clone(),
        )
        .with_attachment_store(attachments.clone());
        if world.mode != "unobserved" {
            writer = writer.with_scenario_hook_store(world.store.clone());
        }
        let result = cucumber::Cucumber::new()
            .steps(Collection::new().given(
                None,
                Regex::new("^work$").unwrap(),
                work,
            ))
            .with_hooks(hooks)
            .with_attachments(attachments)
            .with_source_store(sources)
            .with_writer(writer.normalized())
            .with_default_cli()
            .run(&path)
            .await;
        drop(result);
        world.messages = String::from_utf8(output.clone())
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(
            world
                .messages
                .iter()
                .find_map(|e| e.get("testRunFinished"))
                .unwrap()["success"],
            true
        );
    } else {
        let run = cucumber::Cucumber::new()
            .steps(Collection::new().given(
                None,
                Regex::new("^work$").unwrap(),
                work,
            ))
            .with_hooks(hooks)
            .retries(usize::from(world.mode == "retry"))
            .max_concurrent_scenarios(2)
            .with_writer(
                writer::Basic::new(&mut output, Coloring::Never, 0)
                    .summarized(),
            )
            .with_default_cli();
        let report = if world.mode == "replaced" {
            let seen = world.seen.clone();
            run.before(move |_, _, _, w| {
                let seen = seen.clone();
                Box::pin(async move {
                    w.seen = seen.clone();
                    seen.lock().unwrap().trace.push("legacy-before".into());
                })
            })
            .run(&path)
            .await
        } else {
            run.run(&path).await
        };
        assert_eq!(report.parsing_errors(), 0);
        world.hook_errors = report.hook_errors();
        world.step_errors = report.failed_steps();
    }
    world.output = output;
}
fn names(world: &Acceptance) -> Vec<String> {
    world.seen.lock().unwrap().trace.clone()
}
fn scopes(world: &Acceptance) -> Vec<ScenarioHookScope> {
    world.seen.lock().unwrap().scopes.clone()
}
fn after_scopes(world: &Acceptance) -> Vec<ScenarioHookScope> {
    scopes(world)
        .into_iter()
        .filter(|s| s.definition.kind == ScenarioHookKind::After)
        .collect()
}
fn actual_names(world: &Acceptance) -> Vec<String> {
    world
        .store
        .records()
        .into_iter()
        .filter(|r| {
            matches!(r.disposition, ScenarioHookDisposition::Executed { .. })
        })
        .map(|r| r.definition.name)
        .collect()
}
#[then(
    "selected callbacks execute in registration order and reverse cleanup order"
)]
fn order(w: &mut Acceptance) {
    assert_eq!(
        names(w),
        ["before-one", "before-two", "after-two", "after-one"]
    );
}
#[then("each executed callback has its own definition and invocation identity")]
fn identities(w: &mut Acceptance) {
    let mut ids = HashSet::new();
    let mut defs = HashSet::new();
    for r in w.store.records() {
        assert!(defs.insert(r.definition.id));
        if let ScenarioHookDisposition::Executed { invocation_id, .. } =
            r.disposition
        {
            assert!(ids.insert(invocation_id));
            assert!(scopes(w).iter().any(|s| s.invocation_id == invocation_id
                && s.definition.id == r.definition.id));
        } else {
            panic!("unexpected skipped slot");
        }
    }
    assert_eq!(ids.len(), 4);
}
#[then("recorded timestamps and outcomes describe actual executions")]
fn timing(w: &mut Acceptance) {
    for r in w.store.records() {
        if let ScenarioHookDisposition::Executed {
            started_at,
            finished_at,
            result,
            ..
        } = r.disposition
        {
            assert!(finished_at >= started_at);
            assert!(result.is_ok());
            assert!(
                r.definition
                    .location
                    .path
                    .ends_with("scenario_hook_records.rs")
            );
            assert!(r.definition.location.line > 0);
        }
    }
}
#[then("the failed setup callback and every cleanup callback execute")]
fn setup_cleanup(w: &mut Acceptance) {
    assert_eq!(actual_names(w), ["before-one", "after-two", "after-one"]);
    assert_eq!(w.hook_errors, 1);
}
#[then(
    "later selected setup is recorded as skipped without an invocation identity"
)]
fn skipped_setup(w: &mut Acceptance) {
    let r = w.store.records();
    assert_eq!(r.len(), 4);
    assert!(matches!(
        r[1].disposition,
        ScenarioHookDisposition::Skipped {
            reason: ScenarioHookSkipReason::PreviousFailure,
            ..
        }
    ));
}
#[then("no scenario body executes")]
fn no_body(w: &mut Acceptance) {
    assert_eq!(w.seen.lock().unwrap().bodies, 0);
}
#[then("every selected cleanup callback executes")]
fn cleanup(w: &mut Acceptance) {
    let n = actual_names(w);
    assert!(n.ends_with(&["after-two".into(), "after-one".into()]));
}
#[then("the actual failing callback has its own diagnostic")]
fn diagnostic(w: &mut Acceptance) {
    assert!(w.store.records().iter().any(|r|matches!(&r.disposition,ScenarioHookDisposition::Executed{result:Err(message),..}if message.contains("callback construction failure"))));
}
#[then("the first cleanup callback observes a passing body")]
fn first_pass(w: &mut Acceptance) {
    assert_eq!(
        after_scopes(w)[0].result.as_ref().unwrap().status,
        ScenarioHookStatus::Passed
    );
}
#[then("the next cleanup callback observes the earlier cleanup failure")]
#[then("scoped cleanup observes the earlier cleanup failure")]
fn evolving(w: &mut Acceptance) {
    let s = after_scopes(w);
    let r = s.last().unwrap().result.as_ref().unwrap();
    assert_eq!(r.status, ScenarioHookStatus::Failed);
    assert_eq!(r.previous_hook_failures.len(), 1);
    assert!(
        r.primary_failure
            .as_ref()
            .unwrap()
            .contains("cleanup failed after-two")
    );
}
#[then("both cleanup callbacks retain the original passing body snapshot")]
fn snapshots(w: &mut Acceptance) {
    assert!(after_scopes(w).iter().all(|s| matches!(
        s.body_snapshot,
        Some(ScenarioFinished::StepPassed)
    )));
}
#[then("later cleanup retains the body's first failing diagnostic")]
fn first_failure(w: &mut Acceptance) {
    for s in after_scopes(w) {
        assert!(
            s.result
                .unwrap()
                .primary_failure
                .unwrap()
                .contains("primary body failure")
        );
    }
}
#[then("every individual cleanup failure remains available in records")]
fn cleanup_failure(w: &mut Acceptance) {
    assert_eq!(
        w.store
            .records()
            .iter()
            .filter(|r| r.definition.kind == ScenarioHookKind::After
                && matches!(
                    r.disposition,
                    ScenarioHookDisposition::Executed { result: Err(_), .. }
                ))
            .count(),
        1
    );
}
#[then("legacy cleanup receives its original ScenarioFinished snapshot")]
fn legacy_snapshot(w: &mut Acceptance) {
    assert!(matches!(
        w.seen.lock().unwrap().legacy.as_slice(),
        [ScenarioFinished::StepPassed]
    ));
}
#[then("only matching callback definitions have selected slots")]
fn selected(w: &mut Acceptance) {
    let p = w.store.plans();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].before.len(), 1);
    assert_eq!(p[0].after.len(), 1);
}
#[then(
    "excluded callbacks have neither skipped records nor invocation identities"
)]
fn excluded(w: &mut Acceptance) {
    assert!(
        w.store
            .records()
            .iter()
            .all(|r| !r.definition.name.starts_with("excluded"))
    );
    assert_eq!(w.store.records().len(), 2);
}
#[then("callback definitions remain stable across attempts")]
fn stable_defs(w: &mut Acceptance) {
    let p = w.store.plans();
    assert_eq!(p.len(), 2);
    assert_eq!(
        p[0].before.iter().map(|d| d.id).collect::<Vec<_>>(),
        p[1].before.iter().map(|d| d.id).collect::<Vec<_>>()
    );
}
#[then("every attempt has a fresh World and distinct execution identities")]
fn fresh(w: &mut Acceptance) {
    assert_eq!(w.seen.lock().unwrap().initial_sizes, [0, 0]);
    let p = w.store.plans();
    assert_ne!(p[0].attempt.scenario_id, p[1].attempt.scenario_id);
    let mut ids = HashSet::new();
    for r in w.store.records() {
        if let ScenarioHookDisposition::Executed { invocation_id, .. } =
            r.disposition
        {
            assert!(ids.insert(invocation_id));
        }
    }
    assert_eq!(ids.len(), 8);
}
#[then(
    "scoped cleanup observes failure with retry availability only on the first attempt"
)]
fn retry(w: &mut Acceptance) {
    for s in after_scopes(w) {
        let r = s.result.unwrap();
        assert_eq!(r.retry_if_finished_now, s.attempt.attempt == 0);
        assert_eq!(
            r.status,
            if s.attempt.attempt == 0 {
                ScenarioHookStatus::Failed
            } else {
                ScenarioHookStatus::Passed
            }
        );
    }
}
#[then(
    "each callback scope and record belongs to its actual scenario and attempt"
)]
fn isolated(w: &mut Acceptance) {
    let p = w.store.plans();
    assert_eq!(p.len(), 2);
    assert_ne!(p[0].attempt.scenario_id, p[1].attempt.scenario_id);
    for p in p {
        let records = w.store.records_for(p.attempt.scenario_id);
        assert_eq!(records.len(), 4);
        for r in records {
            assert_eq!(r.attempt.scenario, p.attempt.scenario);
            assert_eq!(r.attempt.rule, p.attempt.rule);
        }
    }
}
#[then(
    "only the legacy before callback and registered cleanup callbacks execute"
)]
fn replacement(w: &mut Acceptance) {
    assert_eq!(names(w), ["legacy-before", "after-two", "after-one"]);
}
#[then("replaced registrations have no selected slots or invocation records")]
fn no_phantom(w: &mut Acceptance) {
    let p = w.store.plans();
    assert!(!p[0].before_stage);
    assert!(p[0].before.is_empty());
    assert!(
        w.store
            .records()
            .iter()
            .all(|r| r.definition.kind == ScenarioHookKind::After)
    );
}
#[then("a handle retained from an earlier callback rejects later writes")]
fn closed(w: &mut Acceptance) {
    assert_eq!(w.seen.lock().unwrap().rejected, 3);
    for h in &w.seen.lock().unwrap().handles {
        assert_eq!(
            h.attach(Attachment::text("late", "text/plain")),
            Err(AttachmentError::ClosedScope)
        );
    }
}
#[then("every selected cleanup callback still executes")]
fn native_empty(w: &mut Acceptance) {
    cleanup(w);
}
#[then("scoped cleanup receives the native failed-setup snapshot")]
fn setup_snapshot(w: &mut Acceptance) {
    for s in after_scopes(w) {
        assert!(matches!(
            s.body_snapshot,
            Some(ScenarioFinished::BeforeHookFailed(_))
        ));
        assert_eq!(s.result.unwrap().status, ScenarioHookStatus::Failed);
    }
}
#[then("registered cleanup slots are skipped without invented invocations")]
fn empty_skipped(w: &mut Acceptance) {
    let records = w.store.records();
    assert_eq!(records.len(), 4);
    assert!(
        records
            .iter()
            .filter(|r| r.definition.kind == ScenarioHookKind::After)
            .all(|r| matches!(
                r.disposition,
                ScenarioHookDisposition::Skipped { .. }
            ))
    );
}
#[then("the initial setup failure remains the run failure")]
fn retained_failure(w: &mut Acceptance) {
    assert_eq!(w.hook_errors, 1);
}
#[then("registered cleanup callbacks run in reverse order")]
fn reverse(w: &mut Acceptance) {
    assert_eq!(
        after_scopes(w)
            .iter()
            .map(|s| s.definition.name.as_str())
            .collect::<Vec<_>>(),
        ["after-two", "after-one"]
    );
}
#[then("their scoped result and retry-decision arguments are absent")]
fn absent_result(w: &mut Acceptance) {
    assert!(after_scopes(w).iter().all(|s| s.result.is_none()));
}
#[then("the plan records present stages with empty callback selections")]
fn empty_plan(w: &mut Acceptance) {
    let p = w.store.plans();
    assert!(p[0].before_stage && p[0].after_stage);
    assert!(p[0].before.is_empty() && p[0].after.is_empty());
    assert_eq!(w.store.initializations().len(), 1);
}
#[then("no callback invocation is invented")]
fn no_records(w: &mut Acceptance) {
    assert!(w.store.records().is_empty());
}
fn attachment_targets(w: &Acceptance) -> HashMap<String, String> {
    w.messages
        .iter()
        .filter_map(|e| e.get("attachment"))
        .map(|a| {
            (
                a["body"].as_str().unwrap().into(),
                a["testStepId"].as_str().unwrap().into(),
            )
        })
        .collect()
}
#[then(
    "evidence from each callback retains that callback's definition identity"
)]
#[then("callback evidence retains its individual callback target")]
fn attachment_identity(w: &mut Acceptance) {
    let targets = attachment_targets(w);
    let steps =
        w.messages.iter().find_map(|e| e.get("testCase")).unwrap()["testSteps"]
            .as_array()
            .unwrap();
    let hooks =
        w.messages.iter().filter_map(|e| e.get("hook")).collect::<Vec<_>>();
    for r in w.store.records() {
        if matches!(r.disposition, ScenarioHookDisposition::Executed { .. }) {
            let target = &targets[&r.definition.name];
            let step =
                steps.iter().find(|s| s["id"] == target.as_str()).unwrap();
            let hook =
                hooks.iter().find(|h| h["id"] == step["hookId"]).unwrap();
            assert_eq!(hook["name"], r.definition.name);
        }
    }
    assert_eq!(targets.values().collect::<HashSet<_>>().len(), 5);
}
#[then("its evidence remains under the native composite hook targets")]
fn composite(w: &mut Acceptance) {
    let t = attachment_targets(w);
    assert_eq!(t["before-one"], t["before-two"]);
    assert_eq!(t["after-one"], t["after-two"]);
    assert_ne!(t["before-one"], t["after-one"]);
}
#[then("constructor evidence retains its native setup target")]
fn constructor_target(w: &mut Acceptance) {
    let t = attachment_targets(w);
    for name in ["before-one", "before-two", "after-two", "after-one"] {
        assert_ne!(t["constructor"], t[name]);
    }
}
#[derive(Debug)]
struct Broken;
impl cucumber::World for Broken {
    type Error = std::io::Error;
    async fn new() -> Result<Self, Self::Error> {
        Err(std::io::Error::other("constructor failed"))
    }
}
async fn run_failed_constructor(w: &mut Acceptance, path: &std::path::Path) {
    let seen = w.seen.clone();
    let hooks = Hooks::<Broken>::new()
        .with_store(w.store.clone())
        .before_scoped("unreached", |_, _| {
            Box::pin(async { panic!("must not execute") })
        })
        .after_scoped("cleanup", move |scope, world| {
            let seen = seen.clone();
            Box::pin(async move {
                assert!(world.is_none());
                seen.lock().unwrap().scopes.push(scope.clone());
            })
        });
    let mut output = Vec::new();
    let report = cucumber::Cucumber::new()
        .with_hooks(hooks)
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .run(path)
        .await;
    w.hook_errors = report.hook_errors();
}
#[then("the constructor failure has its own observation")]
fn init_failure(w: &mut Acceptance) {
    let i = w.store.initializations();
    assert_eq!(i.len(), 1);
    assert!(i[0].result.as_ref().unwrap_err().contains("constructor failed"));
    assert!(i[0].finished_at >= i[0].started_at);
    assert_eq!(w.hook_errors, 1);
}
#[then("selected setup callbacks are skipped because initialization failed")]
fn init_skipped(w: &mut Acceptance) {
    assert!(matches!(
        w.store.records()[0].disposition,
        ScenarioHookDisposition::Skipped {
            reason: ScenarioHookSkipReason::WorldInitializationFailed,
            ..
        }
    ));
}
#[then("selected cleanup callbacks still execute without a World")]
fn cleanup_no_world(w: &mut Acceptance) {
    assert_eq!(scopes(w).len(), 1);
    assert_eq!(scopes(w)[0].definition.name, "cleanup");
}
#[tokio::main]
async fn main() {
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/scenario_hook_records")
        .await;
}
