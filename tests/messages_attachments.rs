//! Phase23 BDD: actual typed attachments consumed by the pinned official Query.
#[path = "support/oracle.rs"]
mod oracle;

use std::{
    collections::HashMap,
    convert::Infallible,
    io::Write as _,
    process::Stdio,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use cucumber::{
    World as _, WriterExt as _,
    attachment::{
        Attachment, AttachmentContext, AttachmentError, AttachmentHandle,
        AttachmentStore,
    },
    given,
    step::{Collection, Context},
    then, when,
};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::{Value, json};

#[derive(Debug, Default, cucumber::World)]
struct Report {
    mode: String,
    output: Vec<u8>,
    messages: Vec<Value>,
    query: Value,
    rejected: usize,
    retained: usize,
    legacy_new: usize,
}
#[derive(Debug, Default)]
struct Shared {
    attempts: Mutex<HashMap<String, usize>>,
    handles: Mutex<Vec<AttachmentHandle>>,
    rejected: AtomicUsize,
    barrier: Option<tokio::sync::Barrier>,
}
#[derive(Debug, Default)]
struct Application {
    attachments: AttachmentContext,
    mode: String,
    name: String,
    attempt: usize,
    shared: Arc<Shared>,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
    async fn new_with_context(
        attachments: AttachmentContext,
    ) -> Result<Self, Self::Error> {
        Ok(Self { attachments, ..Self::default() })
    }
}
fn text(world: &Application, label: &str) {
    world
        .attachments
        .scope()
        .unwrap()
        .attach(Attachment::text(
            format!("{}:{}:{label}", world.name, world.attempt),
            "text/plain",
        ))
        .unwrap();
}
fn act(world: &mut Application, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        if let Some(gate) = &world.shared.barrier {
            gate.wait().await;
        }
        if (world.mode == "late step" && ctx.step.value == "second")
            || (world.mode == "late attempt" && world.attempt == 1)
        {
            let previous =
                world.shared.handles.lock().unwrap().last().unwrap().clone();
            assert_eq!(
                previous
                    .attach(Attachment::text("MUST NOT APPEAR", "text/plain")),
                Err(AttachmentError::ClosedScope)
            );
            world.shared.rejected.fetch_add(1, Ordering::SeqCst);
        }
        let handle = world.attachments.scope().unwrap();
        if world.mode == "payloads" {
            handle
                .attach(
                    Attachment::text(
                        "café\nexact text\u{0000}",
                        "text/custom; charset=utf-8",
                    )
                    .with_file_name("notes ü.txt"),
                )
                .unwrap();
            handle
                .attach(
                    Attachment::bytes(
                        vec![0, 1, 127, 128, 255, 10],
                        "application/octet-stream",
                    )
                    .with_file_name("proof.bin"),
                )
                .unwrap();
        } else {
            text(world, &ctx.step.value);
        }
        world.shared.handles.lock().unwrap().push(handle);
        if matches!(world.mode.as_str(), "retry" | "late attempt")
            && world.attempt == 0
        {
            panic!("deliberate attachment retry");
        }
    })
}

static LEGACY_CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug)]
struct Legacy;
impl cucumber::World for Legacy {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        LEGACY_CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Ok(Self)
    }
}
fn legacy_step(_: &mut Legacy, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}

#[given(expr = "an attachment-aware application in {string} mode")]
fn choose(report: &mut Report, mode: String) {
    report.mode = mode;
}
#[given("an existing application with no attachment-aware constructor")]
fn legacy(report: &mut Report) {
    report.mode = "legacy".into();
}

#[when("its real execution is reported with the shared attachment store")]
async fn execute(report: &mut Report) {
    let names = if report.mode == "concurrent" {
        vec!["left", "right"]
    } else {
        vec!["application"]
    };
    let mut source = "Feature: Typed evidence\n".to_owned();
    for name in names {
        source.push_str(&format!("  Scenario: {name}\n    Given first\n"));
        if report.mode == "late step" {
            source.push_str("    Then second\n");
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("attachments.feature");
    std::fs::write(&path, &source).unwrap();
    let store = AttachmentStore::new();
    let sources = cucumber::parser::SourceStore::new();
    if report.mode == "lazy" {
        let writer = cucumber::writer::Messages::raw_with_source_store(
            &mut report.output,
            sources.clone(),
        )
        .with_attachment_store(store.clone())
        .normalized();
        cucumber::Cucumber::new()
            .steps(Collection::new().given(
                None,
                Regex::new("^first$").unwrap(),
                constructor_step,
            ))
            .with_attachments(store)
            .with_source_store(sources)
            .with_writer(writer)
            .with_default_cli()
            .run(&path)
            .await;
    } else if report.mode == "legacy" {
        LEGACY_CONSTRUCTIONS.store(0, Ordering::SeqCst);
        let writer = cucumber::writer::Messages::raw_with_source_store(
            &mut report.output,
            sources.clone(),
        )
        .with_attachment_store(store.clone())
        .normalized();
        cucumber::Cucumber::new()
            .steps(Collection::new().given(
                None,
                Regex::new("^first$").unwrap(),
                legacy_step,
            ))
            .with_attachments(store)
            .with_source_store(sources)
            .with_writer(writer)
            .with_default_cli()
            .run(&path)
            .await;
        report.legacy_new = LEGACY_CONSTRUCTIONS.load(Ordering::SeqCst);
    } else {
        let shared = Arc::new(Shared {
            barrier: (report.mode == "concurrent")
                .then(|| tokio::sync::Barrier::new(2)),
            ..Shared::default()
        });
        let before_shared = shared.clone();
        let mode = report.mode.clone();
        let hooks = cucumber::runner::StepHooks::new()
            .before("evidence setup", |_, world: &mut Application| {
                Box::pin(async move {
                    if world.mode == "hooks" {
                        text(world, "before-step");
                    }
                })
            })
            .after("evidence cleanup", |_, _, world: &mut Application| {
                Box::pin(async move {
                    if world.mode == "hooks" {
                        text(world, "after-step");
                    }
                })
            });
        let steps = Collection::new()
            .given(None, Regex::new("^first$").unwrap(), act)
            .then(None, Regex::new("^second$").unwrap(), act);
        let writer = cucumber::writer::Messages::raw_with_source_store(
            &mut report.output,
            sources.clone(),
        )
        .with_attachment_store(store.clone())
        .normalized();
        let run = cucumber::Cucumber::new()
            .steps(steps)
            .with_attachments(store)
            .with_source_store(sources)
            .with_step_hooks(hooks)
            .before(move |_, _, scenario, world| {
                let shared = before_shared.clone();
                let mode = mode.clone();
                Box::pin(async move {
                    world.shared = shared;
                    world.mode = mode;
                    world.name = scenario.name.clone();
                    world.attempt = {
                        let mut attempts =
                            world.shared.attempts.lock().unwrap();
                        let n = attempts.entry(world.name.clone()).or_default();
                        let current = *n;
                        *n += 1;
                        current
                    };
                    if world.mode == "hooks" {
                        text(world, "before-scenario");
                    }
                })
            })
            .after(|_, _, _, _, world| {
                Box::pin(async move {
                    if let Some(world) = world {
                        if world.mode == "hooks" {
                            text(world, "after-scenario");
                        }
                        if world.mode == "completed" {
                            world
                                .shared
                                .handles
                                .lock()
                                .unwrap()
                                .push(world.attachments.scope().unwrap());
                        }
                    }
                })
            })
            .retries(usize::from(matches!(
                report.mode.as_str(),
                "retry" | "late attempt"
            )))
            .max_concurrent_scenarios(2)
            .with_writer(writer)
            .with_default_cli()
            .run(&path);
        tokio::time::timeout(std::time::Duration::from_secs(20), run)
            .await
            .expect("attachment run must not deadlock");
        let handles = shared.handles.lock().unwrap();
        report.retained = handles.len();
        for handle in handles.iter() {
            assert_eq!(
                handle
                    .attach(Attachment::text("MUST NOT APPEAR", "text/plain")),
                Err(AttachmentError::ClosedScope)
            );
        }
        report.rejected = shared.rejected.load(Ordering::SeqCst);
    }
    report.messages = String::from_utf8(report.output.clone())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut child = oracle::command("tests/interop/attachments.mjs")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&report.output).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "official Query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    report.query = serde_json::from_slice(&output.stdout).unwrap();
}
fn attachments(report: &Report) -> Vec<&Value> {
    report.messages.iter().filter_map(|e| e.get("attachment")).collect()
}
fn attempts(report: &Report) -> &Vec<Value> {
    report.query["attempts"].as_array().unwrap()
}
fn query_attachments(attempt: &Value) -> Vec<&Value> {
    attempt["steps"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|step| step["attachments"].as_array().unwrap())
        .collect()
}
fn successful(report: &Report) {
    assert_eq!(
        report.messages.iter().find_map(|e| e.get("testRunFinished")).unwrap()
            ["success"],
        true
    );
}

#[then(
    "official Query retrieves exact text and binary evidence with their \
        media types and file names"
)]
fn payloads(report: &mut Report) {
    successful(report);
    let values = query_attachments(&attempts(report)[0]);
    assert_eq!(values.len(), 2);
    let text =
        values.iter().find(|a| a["contentEncoding"] == "IDENTITY").unwrap();
    assert_eq!(text["body"], "café\nexact text\u{0000}");
    assert_eq!(text["mediaType"], "text/custom; charset=utf-8");
    assert_eq!(text["fileName"], "notes ü.txt");
    let bytes =
        values.iter().find(|a| a["contentEncoding"] == "BASE64").unwrap();
    assert_eq!(bytes["decoded"], json!([0, 1, 127, 128, 255, 10]));
    assert_eq!(bytes["mediaType"], "application/octet-stream");
    assert_eq!(bytes["fileName"], "proof.bin");
}
#[then(
    "official Query retrieves evidence under the actual scenario hook or step"
)]
fn hook_scopes(report: &mut Report) {
    successful(report);
    let steps = attempts(report)[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0]["hookType"], "BEFORE_TEST_CASE");
    assert_eq!(steps[2]["hookType"], "AFTER_TEST_CASE");
    assert_eq!(steps[1]["text"], "first");
    for (step, labels) in [
        (&steps[0], vec!["before-scenario"]),
        (&steps[1], vec!["before-step", "first", "after-step"]),
        (&steps[2], vec!["after-scenario"]),
    ] {
        let actual = step["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["body"].as_str().unwrap())
            .collect::<Vec<_>>();
        let expected = labels
            .iter()
            .map(|label| format!("application:0:{label}"))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}
#[then("each concurrent scenario retrieves only its own evidence")]
fn concurrent(report: &mut Report) {
    successful(report);
    assert_eq!(attempts(report).len(), 2);
    for attempt in attempts(report) {
        let values = query_attachments(attempt);
        assert_eq!(values.len(), 1);
        assert_eq!(
            values[0]["body"],
            format!("{}:0:first", attempt["name"].as_str().unwrap())
        );
    }
}
#[then("each attempt retrieves its own evidence and the final attempt passes")]
fn retries(report: &mut Report) {
    successful(report);
    assert_eq!(attempts(report).len(), 2);
    for (index, attempt) in attempts(report).iter().enumerate() {
        assert_eq!(attempt["attempt"], index);
        let values = query_attachments(attempt);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["body"], format!("application:{index}:first"));
        let step = attempt["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s.get("text").is_some())
            .unwrap();
        assert_eq!(
            step["status"],
            if index == 0 { "FAILED" } else { "PASSED" }
        );
    }
}
#[then(
    "the old step handle rejects the write while the current scope accepts \
        evidence"
)]
fn old_step(report: &mut Report) {
    successful(report);
    assert_eq!(report.rejected, 1);
    let values = query_attachments(&attempts(report)[0]);
    assert_eq!(values.len(), 2);
    assert_eq!(values[0]["body"], "application:0:first");
    assert_eq!(values[1]["body"], "application:0:second");
    assert_ne!(values[0]["testStepId"], values[1]["testStepId"]);
}
#[then(
    "the old attempt handle rejects the write while the retry accepts \
        evidence"
)]
fn old_attempt(report: &mut Report) {
    assert_eq!(report.rejected, 1);
    retries(report);
}
#[then("all retained scope handles reject writes after completion")]
fn completed(report: &mut Report) {
    successful(report);
    assert_eq!(report.retained, 2);
    assert_eq!(attachments(report).len(), 1);
}
#[then("its original World constructor and steps still run successfully")]
fn old_world(report: &mut Report) {
    successful(report);
    assert_eq!(report.legacy_new, 1);
    assert_eq!(attempts(report)[0]["steps"][0]["status"], "PASSED");
}
#[then("the report contains no invented attachments")]
fn no_attachments(report: &mut Report) {
    assert!(attachments(report).is_empty());
}

#[then(
    "all typed attachments satisfy the pinned schema and execution reference \
        graph"
)]
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
    let mut active: HashMap<String, String> = HashMap::new();
    let mut retrieved = 0;
    for message in &report.messages {
        let errors = SCHEMA
            .iter_errors(message)
            .map(|e| e.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
        if let Some(start) = message.get("testStepStarted") {
            active.insert(
                start["testCaseStartedId"].as_str().unwrap().into(),
                start["testStepId"].as_str().unwrap().into(),
            );
        }
        if let Some(attachment) = message.get("attachment") {
            assert_eq!(
                active
                    .get(attachment["testCaseStartedId"].as_str().unwrap())
                    .map(String::as_str),
                attachment["testStepId"].as_str()
            );
            assert!(attachment.get("timestamp").is_some());
            assert_ne!(attachment["body"], "MUST NOT APPEAR");
            let same_scope = |event: &Value| {
                event["testCaseStartedId"] == attachment["testCaseStartedId"]
                    && event["testStepId"] == attachment["testStepId"]
            };
            let started = report
                .messages
                .iter()
                .filter_map(|m| m.get("testStepStarted"))
                .find(|event| same_scope(event))
                .unwrap();
            let finished = report
                .messages
                .iter()
                .filter_map(|m| m.get("testStepFinished"))
                .find(|event| same_scope(event))
                .unwrap();
            let timestamp = |value: &Value| {
                (
                    value["seconds"].as_u64().unwrap(),
                    value["nanos"].as_u64().unwrap(),
                )
            };
            assert!(
                timestamp(&started["timestamp"])
                    <= timestamp(&attachment["timestamp"])
            );
            assert!(
                timestamp(&attachment["timestamp"])
                    <= timestamp(&finished["timestamp"])
            );
        }
        if let Some(end) = message.get("testStepFinished") {
            assert_eq!(
                active
                    .remove(end["testCaseStartedId"].as_str().unwrap())
                    .as_deref(),
                end["testStepId"].as_str()
            );
        }
    }
    assert!(active.is_empty());
    for attempt in attempts(report) {
        retrieved += query_attachments(attempt).len();
    }
    assert_eq!(
        retrieved,
        attachments(report).len(),
        "official Query retrieves every typed attachment exactly once"
    );
}
#[tokio::main]
async fn main() {
    Report::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/messages_attachments")
        .await;
}

#[derive(Debug, Default)]
struct ConstructorApplication {
    attachments: AttachmentContext,
}
impl cucumber::World for ConstructorApplication {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
    async fn new_with_context(
        attachments: AttachmentContext,
    ) -> Result<Self, Self::Error> {
        attachments
            .scope()
            .unwrap()
            .attach(Attachment::text("constructor", "text/plain"))
            .unwrap();
        Ok(Self { attachments })
    }
}
fn constructor_step(
    world: &mut ConstructorApplication,
    _: Context,
) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        world
            .attachments
            .scope()
            .unwrap()
            .attach(Attachment::text("body", "text/plain"))
            .unwrap();
    })
}
#[then(
    "official Query retrieves constructor and body evidence under the first \
        step"
)]
fn constructor_scope(report: &mut Report) {
    successful(report);
    let steps = attempts(report)[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["text"], "first");
    let values = query_attachments(&attempts(report)[0]);
    assert_eq!(values.len(), 2);
    assert_eq!(values[0]["body"], "constructor");
    assert_eq!(values[1]["body"], "body");
    assert_eq!(values[0]["testStepId"], values[1]["testStepId"]);
}
