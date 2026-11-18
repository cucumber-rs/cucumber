//! Phase18 BDD: actual tracing events represented as case-attempt log attachments.
use std::{collections::HashMap, path::Path, io::Write as _, process::{Command, Stdio}, sync::{Arc, LazyLock, Mutex, OnceLock}, time::UNIX_EPOCH};
use cucumber::{given, when, then, World as _, WriterExt as _, event, parser, Event};
use serde_json::{Value, json};
use tracing_subscriber::{fmt::{FmtContext, FormatEvent, FormatFields, format::{DefaultFields, Writer}}, layer::SubscriberExt as _, registry::LookupSpan};

#[derive(Debug, Default, cucumber::World)]
struct Report { mode: String, messages: Vec<Value>, observed: Vec<Value>, delays: usize }
#[derive(Debug, Default, cucumber::World)]
struct Execution { name: String, attempt: usize }

static MODE: OnceLock<String> = OnceLock::new();
static DELAYS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
fn ordered(mode: &str) -> bool { mode.starts_with("ordered ") }
fn parallel_retry(mode: &str) -> bool { mode == "concurrent" || ordered(mode) }
#[derive(Debug)]
struct YieldClock;
impl cucumber::runner::TimeoutClock for YieldClock {
    fn sleep(&self, _: std::time::Duration) -> futures::future::BoxFuture<'static, ()> {
        DELAYS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async { tokio::task::yield_now().await; })
    }
}
static COUNTS: LazyLock<Mutex<HashMap<String, usize>>> = LazyLock::new(Default::default);
static GATE: OnceLock<tokio::sync::Barrier> = OnceLock::new();

#[given("emit application logs")]
async fn emit_logs(world: &mut Execution) {
    let mode = MODE.get().unwrap();
    if parallel_retry(mode) || mode == "fanout" {
        GATE.get().unwrap().wait().await;
    }
    tracing::info!("log:{}:{}:body café\nsecond line", world.name, world.attempt);
    if mode == "fanout" {
        if world.name == "left" { tracing::info!(parent: None, "unscoped native broadcast"); }
        GATE.get().unwrap().wait().await;
    }
    if (mode == "retry" && world.attempt == 0)
        || (parallel_retry(mode) && (world.attempt == 0 || world.name == "exhausted"))
    {
        panic!("deliberate logged failure");
    }
}

struct BodyFormat;
impl<S, N> FormatEvent<S, N> for BodyFormat
where S: tracing::Subscriber + for<'a> LookupSpan<'a>, N: for<'a> FormatFields<'a> + 'static {
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &tracing::Event<'_>) -> std::fmt::Result {
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

#[derive(Debug)]
struct LogRecorder { logs: Arc<Mutex<Vec<Value>>>, active: Option<&'static str> }
impl cucumber::Writer<Execution> for LogRecorder {
    type Cli = cucumber::cli::Empty;
    async fn handle_event(&mut self, input: parser::Result<Event<event::Cucumber<Execution>>>, _: &Self::Cli) {
        let (ev, meta) = input.expect("generated feature parses").split();
        let (scenario, ev) = match ev {
            event::Cucumber::Feature(_, event::Feature::Scenario(scenario, ev)) => (scenario, ev),
            _ => return,
        };
        match ev.event {
            event::Scenario::Log(body) => {
                let elapsed = meta.at.duration_since(UNIX_EPOCH).unwrap();
                self.logs.lock().unwrap().push(json!({
                    "name": scenario.name, "attempt": ev.retries.map_or(0, |r|r.current), "body": body,
                    "timestamp": {"seconds": elapsed.as_secs(), "nanos": elapsed.subsec_nanos()},
                    "observedActive": self.active
                }));
            }
            event::Scenario::Step(_, event::Step::Started) => self.active = Some("step"),
            event::Scenario::Hook(_, event::Hook::Started) => self.active = Some("hook"),
            event::Scenario::Step(_, _) | event::Scenario::Hook(_, _) | event::Scenario::Finished => self.active = None,
            _ => {},
        }
    }
}

async fn produce(mode: String, result_path: &Path) {
    MODE.set(mode.clone()).unwrap();
    if parallel_retry(&mode) || mode == "fanout" { GATE.set(tokio::sync::Barrier::new(2)).unwrap(); }
    let names = match mode.as_str() {
        "concurrent" | "ordered retries" | "ordered retry delay" => vec!["recovered", "exhausted"],
        "fanout" => vec!["left", "right"],
        _ => vec!["application"],
    };
    let mut source = "Feature: Native application logs\n".to_owned();
    for name in names { source.push_str(&format!("  Scenario: {name}\n    Given emit application logs\n")); }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("logs.feature");
    std::fs::write(&path, &source).unwrap();
    let logs = Arc::new(Mutex::new(Vec::new()));
    let store = parser::SourceStore::new();
    let mut output = Vec::new();
    let writer = cucumber::writer::Messages::raw_with_source_store(&mut output, store.clone())
        .tee::<Execution, _>(LogRecorder { logs: logs.clone(), active: None }).normalized();
    let run = Execution::cucumber().with_source_store(store)
        .max_concurrent_scenarios(2)
        .retries(if mode == "retry" || parallel_retry(&mode) { 1 } else { 0 })
        .before(|_, _, scenario, world| Box::pin(async move {
            world.name = scenario.name.clone();
            let attempt = { let mut counts = COUNTS.lock().unwrap(); let count = counts.entry(world.name.clone()).or_default(); let attempt = *count; *count += 1; attempt };
            world.attempt = attempt;
            if matches!(MODE.get().unwrap().as_str(), "hooks" | "retry") || parallel_retry(MODE.get().unwrap()) {
                tracing::info!("log:{}:{}:before", world.name, world.attempt);
            }
        }))
        .after(|_, _, _, _, world| Box::pin(async move {
            if let Some(world) = world {
                if matches!(MODE.get().unwrap().as_str(), "hooks" | "retry") || parallel_retry(MODE.get().unwrap()) {
                    tracing::info!("log:{}:{}:after", world.name, world.attempt);
                }
            }
        }))
        .with_writer(writer).with_default_cli();
    let run = if ordered(&mode) {
        run.scheduling_options(cucumber::runner::SchedulingOptions::new(cucumber::runner::ScenarioOrder::Defined)
            .parallel(2).with_retry_clock(Arc::new(YieldClock)))
    } else { run };
    let run = if mode == "ordered retry delay" { run.retry_after(std::time::Duration::from_secs(1)) } else { run };
    let run = run.configure_and_init_tracing(DefaultFields::new(), BodyFormat, |layer| tracing_subscriber::registry().with(layer))
        .run(&path);
    tokio::time::timeout(std::time::Duration::from_secs(20), run).await.expect("tracing runner must not deadlock");
    let messages = String::from_utf8(output).unwrap().lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap()).collect::<Vec<_>>();
    std::fs::write(result_path, serde_json::to_vec(&json!({"messages": messages, "observed": *logs.lock().unwrap(), "source": source, "delays": DELAYS.load(std::sync::atomic::Ordering::SeqCst)})).unwrap()).unwrap();
}

#[given(expr = "a native tracing application in {string} mode")]
fn select_mode(report: &mut Report, mode: String) { report.mode = mode; }

#[when("the application reports through Messages")]
fn execute(report: &mut Report) {
    // Cucumber initializes a global subscriber. Each actual application run gets
    // its own process; the outer BDD harness never installs or fabricates traces.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result.json");
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--tracing-application").arg(&report.mode).arg(&path)
        .output().expect("launch real tracing application");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let result: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    report.messages = result["messages"].as_array().unwrap().clone();
    report.observed = result["observed"].as_array().unwrap().clone();
    report.delays = result["delays"].as_u64().unwrap() as usize;
    let sources = entries(report, "source");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["data"], result["source"]);
}
fn entries<'a>(report: &'a Report, kind: &str) -> Vec<&'a Value> {
    report.messages.iter().filter_map(|message|message.get(kind)).collect()
}
fn identity<'a>(report: &'a Report, started_id: &Value) -> (&'a str, u64) {
    let start = entries(report,"testCaseStarted").into_iter().find(|s| &s["id"] == started_id).unwrap();
    let case = entries(report,"testCase").into_iter().find(|c|c["id"] == start["testCaseId"]).unwrap();
    let pickle = entries(report,"pickle").into_iter().find(|p|p["id"] == case["pickleId"]).unwrap();
    (pickle["name"].as_str().unwrap(), start["attempt"].as_u64().unwrap())
}
#[then("every observed native log has one exact case-attempt attachment")]
fn exact_logs(report: &mut Report) {
    let attachments = entries(report,"attachment");
    assert!(!report.observed.is_empty());
    assert_eq!(attachments.len(), report.observed.len());
    let mut unmatched = report.observed.clone();
    for attachment in attachments {
        let (name, attempt) = identity(report, &attachment["testCaseStartedId"]);
        let index = unmatched.iter().position(|log|
            log["name"] == name && log["attempt"] == attempt
            && log["body"] == attachment["body"] && log["timestamp"] == attachment["timestamp"])
            .unwrap_or_else(|| panic!("attachment {attachment} did not match observed events {unmatched:?}"));
        unmatched.remove(index);
        assert_eq!(attachment["contentEncoding"], "IDENTITY");
        assert_eq!(attachment["mediaType"], "text/x.cucumber.log+plain");
    }
    assert!(unmatched.is_empty());
}
#[then("log attachments have no inferred step or hook identity")]
fn no_inference(report: &mut Report) {
    for attachment in entries(report,"attachment") {
        for absent in ["testStepId","testRunHookStartedId","testRunStartedId","fileName","url","source"] {
            assert!(attachment.get(absent).is_none(), "fabricated {absent}: {attachment}");
        }
    }
}
#[then(expr = "the {string} run retains its expected logs and final outcome")]
fn outcomes(report: &mut Report, mode: String) {
    let (attempts, count, success) = match mode.as_str() {
        "plain" => (1,1,true), "hooks" => (1,3,true), "retry" => (2,6,true),
        "concurrent" | "ordered retries" | "ordered retry delay" => (4,12,false), "fanout" => (2,4,true), _ => unreachable!(),
    };
    assert_eq!(entries(report,"testCaseStarted").len(), attempts);
    assert_eq!(report.observed.len(), count);
    assert_eq!(entries(report,"testRunFinished")[0]["success"], success);
    let starts = entries(report,"testCaseStarted");
    for start in starts {
        let (name, attempt) = identity(report, &start["id"]);
        let matching = report.observed.iter().filter(|l|l["name"] == name && l["attempt"] == attempt).collect::<Vec<_>>();
        let body = format!("log:{name}:{attempt}:body café\nsecond line\n");
        assert_eq!(matching.iter().filter(|l|l["body"] == body).count(), 1);
        if matches!(mode.as_str(),"hooks"|"retry") || parallel_retry(&mode) {
            for stage in ["before","after"] {
                assert_eq!(matching.iter().filter(|l|l["body"] == format!("log:{name}:{attempt}:{stage}\n")).count(),1);
            }
        }
        if mode == "fanout" {
            assert_eq!(matching.iter().filter(|l|l["body"] == "unscoped native broadcast\n").count(),1);
        }
    }
    if mode == "retry" {
        let after = report.observed.iter().find(|l|l["attempt"] == 0 && l["body"] == "log:application:0:after\n").unwrap();
        assert_eq!(after["observedActive"], "step", "native delayed failure demonstrates why active-step inference is unsafe");
    }
}
#[then("all log report envelopes satisfy the pinned Messages schema")]
fn schema(report: &mut Report) {
    static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| jsonschema::validator_for(
        &serde_json::from_str(include_str!("fixtures/messages/messages-34.2.1.schema.json")).unwrap()).unwrap());
    let mut active: Option<&Value> = None;
    for message in &report.messages {
        let errors = SCHEMA.iter_errors(message).map(|e|e.to_string()).collect::<Vec<_>>();
        assert!(errors.is_empty(), "{message}: {errors:?}");
        if let Some(start) = message.get("testCaseStarted") { assert!(active.replace(&start["id"]).is_none()); }
        if let Some(attachment) = message.get("attachment") { assert_eq!(active, Some(&attachment["testCaseStartedId"])); }
        if message.get("testCaseFinished").is_some() { assert!(active.take().is_some()); }
    }
    assert!(active.is_none());
}
#[then("official Query resolves every ordered attempt and its log identities")]
fn ordered_query(report: &mut Report) {
    let mut child = Command::new("node").arg("tests/interop/ordered-logs.mjs")
        .current_dir(env!("CARGO_MANIFEST_DIR")).stdin(std::process::Stdio::piped())
        .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    { let mut input = child.stdin.take().unwrap(); for message in &report.messages { writeln!(input, "{message}").unwrap(); } }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["attempts"].as_array().unwrap().len(), 4);
    assert_eq!(result["finals"].as_array().unwrap().len(), 2);
    for (name, expected) in [("recovered", "PASSED"), ("exhausted", "FAILED")] {
        let attempts = result["attempts"].as_array().unwrap().iter().filter(|a| a["name"] == name).collect::<Vec<_>>();
        assert_eq!(attempts.iter().map(|a|a["attempt"].as_u64().unwrap()).collect::<Vec<_>>(), [0,1]);
        assert_eq!(attempts[0]["willBeRetried"], true);
        assert_eq!(attempts[1]["willBeRetried"], false);
        assert_eq!(attempts[0]["status"], "FAILED");
        assert_eq!(attempts[1]["status"], expected);
        let final_case = result["finals"].as_array().unwrap().iter().find(|a|a["name"]==name).unwrap();
        assert_eq!(final_case["attempt"], 1); assert_eq!(final_case["status"], expected);
        for attempt in attempts {
            let number = attempt["attempt"].as_u64().unwrap();
            assert_eq!(attempt["logs"].as_array().unwrap().len(), 3);
            for body in attempt["logs"].as_array().unwrap() {
                assert!(body.as_str().unwrap().starts_with(&format!("log:{name}:{number}:")));
            }
        }
    }
    assert_eq!(result["counts"]["PASSED"], 1); assert_eq!(result["counts"]["FAILED"], 1);
}
#[then(expr = "the ordered retry clock records {int} delays")]
fn ordered_delays(report: &mut Report, expected: usize) { assert_eq!(report.delays, expected); }
#[tokio::main]
async fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--tracing-application") {
        produce(args[2].clone(), Path::new(&args[3])).await;
    } else {
        Report::cucumber().max_concurrent_scenarios(1).fail_on_skipped()
            .run_and_exit("tests/features/messages_logs").await;
    }
}
