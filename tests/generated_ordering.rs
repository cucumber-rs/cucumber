//! Phase41: observed generated seeds and actual application replay.
use cucumber::{
    StatsWriter as _, World as _, WriterExt as _,
    feature::Ext as _,
    given, parser,
    runner::{
        Hooks, OrderingRecord, OrderingStore, ScenarioOrder, SchedulingOptions,
    },
    step::{Collection, Context},
    then, when,
    writer::{self, Coloring},
};
use futures::{StreamExt as _, future::LocalBoxFuture};
use regex::Regex;
use serde_json::{Value, json};
use std::{
    io::Write as _,
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

static WORLDS: AtomicUsize = AtomicUsize::new(0);
#[derive(Debug, Default)]
struct Application;
impl cucumber::World for Application {
    type Error = std::convert::Infallible;
    async fn new() -> Result<Self, Self::Error> {
        WORLDS.fetch_add(1, Ordering::SeqCst);
        Ok(Self)
    }
}
fn body(_: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}
struct InputParser(Vec<gherkin::Feature>);
impl cucumber::Parser<()> for InputParser {
    type Cli = cucumber::cli::Empty;
    type Output = futures::stream::LocalBoxStream<
        'static,
        parser::Result<gherkin::Feature>,
    >;
    fn parse(self, _: (), _: Self::Cli) -> Self::Output {
        futures::stream::iter(self.0.into_iter().map(Ok)).boxed_local()
    }
}
fn fixtures() -> Vec<gherkin::Feature> {
    [
        "Feature: First\n Scenario: first\n  Given work\n Rule: Accounts\n  Scenario Outline: row <number>\n   Given work\n   Examples:\n    | number |\n    | one |\n    | two |\n    | three |\n",
        "Feature: Second\n Scenario: last\n  Given work\n Rule: Final\n  Scenario: final\n   Given work\n",
    ].into_iter().map(|source| gherkin::Feature::parse(source, Default::default()).unwrap().expand_examples().unwrap()).collect()
}
fn baseline(filtered: bool) -> Vec<String> {
    fixtures()
        .iter()
        .flat_map(|f| {
            f.scenarios.iter().chain(f.rules.iter().flat_map(|r| &r.scenarios))
        })
        .filter(|s| !filtered || selected(&s.name))
        .map(|s| s.name.clone())
        .collect()
}
fn selected(name: &str) -> bool {
    name != "row two" && name != "last"
}
fn record_json(record: &OrderingRecord) -> Value {
    let order = match &record.requested_order {
        ScenarioOrder::Defined => "defined",
        ScenarioOrder::Reverse => "reverse",
        ScenarioOrder::Random { .. } => "explicit",
        ScenarioOrder::RandomGenerated => "generated",
        _ => panic!("unexpected requested order"),
    };
    json!({"run_id":record.run_id, "order":order, "seed":record.realized_seed,
        "count":record.case_count, "at":record.at.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string()})
}
async fn application(
    options: Option<SchedulingOptions>,
    store: OrderingStore,
    filtered: bool,
    empty_plan: bool,
    timeline: Arc<Mutex<Vec<Value>>>,
) -> Value {
    let worlds_before = WORLDS.load(Ordering::SeqCst);
    let starts = Arc::new(Mutex::new(Vec::<String>::new()));
    let started = starts.clone();
    let expected_record = options.is_some();
    let hooks = Hooks::new().before("observe actual dispatch", move |_, _, scenario, _: &mut Application| {
        let record = store.latest();
        assert_eq!(record.is_some(), expected_record, "ordering must complete before the first callback");
        started.lock().unwrap().push(scenario.name.clone());
        timeline.lock().unwrap().push(json!({"kind":"start", "name":scenario.name, "run_id":record.as_ref().map(|r|r.run_id)}));
        eprintln!("CASE_START:{}", scenario.name);
        Box::pin(async {})
    });
    let mut app =
        cucumber::Cucumber::<Application, _, std::path::PathBuf, _, _>::new()
            .with_parser(InputParser(fixtures()))
            .steps(Collection::new().given(
                None,
                Regex::new("^work$").unwrap(),
                body,
            ))
            .with_hooks(hooks)
            .max_concurrent_scenarios(1)
            .with_writer(
                writer::Basic::raw(std::io::sink(), Coloring::Never, 0)
                    .normalized()
                    .summarized(),
            );
    if let Some(options) = options {
        app = app.scheduling_options(options);
    }
    let report = app
        .with_default_cli()
        .filter_run((), move |_, _, scenario| {
            !empty_plan && (!filtered || selected(&scenario.name))
        })
        .await;
    let names = starts.lock().unwrap().clone();
    json!({"worlds": WORLDS.load(Ordering::SeqCst) - worlds_before, "names":names, "passed":report.passed_steps(), "failed":report.failed_steps(), "errors":report.parsing_errors(), "failed_run":report.execution_has_failed()})
}
async fn child(request: Value) -> Value {
    let mode = request["mode"].as_str().unwrap();
    let filtered = matches!(mode, "filtered" | "filtered_replay");
    let empty_plan = mode == "filtered_empty";
    let store = OrderingStore::new();
    let timeline = Arc::new(Mutex::new(Vec::<Value>::new()));
    let order = match mode {
        "empty" => ScenarioOrder::Random { seed: String::new() },
        "replay" | "filtered_replay" => ScenarioOrder::Random {
            seed: request["seed"].as_str().unwrap().to_owned(),
        },
        "defined" => ScenarioOrder::Defined,
        "reverse" => ScenarioOrder::Reverse,
        _ => ScenarioOrder::RandomGenerated,
    };
    let mut options = SchedulingOptions::new(order)
        .parallel(0)
        .with_ordering_store(store.clone());
    if matches!(mode, "custom" | "reuse") {
        let reported = timeline.clone();
        let stored = options.ordering_store();
        options = options.on_generated_seed(move |record| {
            assert_eq!(
                stored.latest().as_ref(),
                Some(record),
                "the report must refer to the already-stored realization"
            );
            reported
                .lock()
                .unwrap()
                .push(json!({"kind":"report", "record":record_json(record)}));
        });
    }
    let mut runs = Vec::new();
    for _ in 0..if mode == "reuse" { 2 } else { 1 } {
        runs.push(
            application(
                (mode != "native").then(|| options.clone()),
                store.clone(),
                filtered,
                empty_plan,
                timeline.clone(),
            )
            .await,
        );
    }
    let records: Vec<_> = store.records().iter().map(record_json).collect();
    let timeline = timeline.lock().unwrap().clone();
    json!({"records":records, "runs":runs, "timeline":timeline, "baseline":if empty_plan { Vec::new() } else { baseline(filtered) }})
}
fn invoke(mode: &str, seed: Option<&str>) -> (Value, String) {
    let output = Command::new(std::env::current_exe().unwrap())
        .env(
            "CUCUMBER_GENERATED_ORDER_CHILD",
            json!({"mode":mode, "seed":seed}).to_string(),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    for run in value["runs"].as_array().unwrap() {
        assert_eq!(run["failed"], 0);
        assert_eq!(run["errors"], 0);
        assert_eq!(run["failed_run"], false);
        assert_eq!(run["passed"], value["baseline"].as_array().unwrap().len());
    }
    (value, String::from_utf8(output.stderr).unwrap())
}
fn oracle(items: &Value, seed: &str) -> Value {
    let mut child = Command::new("node")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/shuffle37/oracle.mjs"
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pinned shuffle oracle needs Node");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(json!({"items":items,"seed":seed}).to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    result: Value,
    replay: Value,
    stderr: String,
    replay_stderr: String,
}
#[given("an application with explicit generated random ordering")]
fn generated(w: &mut Acceptance) {
    w.mode = "generated".into();
}
#[given("generated ordering over filtered features and expanded outline rows")]
fn filtered(w: &mut Acceptance) {
    w.mode = "filtered".into();
}
#[given("generated ordering whose selection excludes every case")]
fn empty_plan(w: &mut Acceptance) {
    w.mode = "filtered_empty".into();
}
#[given("generated ordering with an application-owned seed report sink")]
fn custom(w: &mut Acceptance) {
    w.mode = "custom".into();
}
#[given("cloned generated-order options sharing an ordering store")]
fn reuse(w: &mut Acceptance) {
    w.mode = "reuse".into();
}
#[given("an application with an explicit empty-string random seed")]
fn empty(w: &mut Acceptance) {
    w.mode = "empty".into();
}
#[given(expr = "an application with explicit {string} ordering")]
fn explicit(w: &mut Acceptance, order: String) {
    w.mode = order;
}
#[given("an application without explicit scheduling options")]
fn native(w: &mut Acceptance) {
    w.mode = "native".into();
}
#[when("the complete selected plan is permuted and executed")]
#[when("its selected cases are executed")]
#[when("two separate selected plans are executed with those options")]
fn run(w: &mut Acceptance) {
    (w.result, w.stderr) = invoke(&w.mode, None);
}
#[when("its observed seed is replayed as an explicit seed")]
fn replay(w: &mut Acceptance) {
    let seed = w.result["records"][0]["seed"].as_str().unwrap();
    (w.replay, w.replay_stderr) = invoke(
        if w.mode == "filtered" { "filtered_replay" } else { "replay" },
        Some(seed),
    );
}
fn seed(value: &Value) -> &str {
    value["records"][0]["seed"].as_str().unwrap()
}
fn valid_seed(value: &str) {
    assert!(!value.is_empty());
    assert!(value.bytes().all(|b| b.is_ascii_digit()));
    let numeric = value.parse::<u32>().unwrap();
    assert!(numeric < 1_000_000);
    assert_eq!(numeric.to_string(), value);
}
#[then("one ordering record contains a decimal seed between 0 and 999999")]
fn generated_record(w: &mut Acceptance) {
    assert_eq!(w.result["records"].as_array().unwrap().len(), 1);
    valid_seed(seed(&w.result));
    assert_eq!(w.result["records"][0]["order"], "generated");
}
#[then("the record describes the actual selected plan and permutation")]
fn recorded_plan(w: &mut Acceptance) {
    let record = &w.result["records"][0];
    assert_eq!(record["count"], w.result["baseline"].as_array().unwrap().len());
    assert!(record["at"].as_str().unwrap().parse::<u128>().unwrap() > 0);
    assert_eq!(
        w.result["runs"][0]["names"],
        oracle(&w.result["baseline"], seed(&w.result))
    );
    assert!(
        w.result["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["run_id"] == record["run_id"])
    );
}
#[then(
    "the generated seed is reported exactly once before the first case starts"
)]
fn report_before(w: &mut Acceptance) {
    let expected = format!("Random order using seed: {}", seed(&w.result));
    assert_eq!(w.stderr.matches("Random order using seed:").count(), 1);
    assert!(
        w.stderr.find(&expected).unwrap()
            < w.stderr.find("CASE_START:").unwrap()
    );
}
#[then(
    "both executions match the pinned JavaScript shuffle oracle for that seed"
)]
#[then(
    "both executions match the pinned JavaScript shuffle oracle for that selected plan"
)]
fn replayed(w: &mut Acceptance) {
    let expected = oracle(&w.result["baseline"], seed(&w.result));
    assert_eq!(w.result["runs"][0]["names"], expected);
    assert_eq!(w.replay["runs"][0]["names"], expected);
    assert_eq!(w.replay["records"][0]["seed"], w.result["records"][0]["seed"]);
    assert_eq!(w.replay["records"][0]["order"], "explicit");
    assert!(!w.replay_stderr.contains("Random order using seed:"));
}
#[then("every selected case executes exactly once in each run")]
fn exactly_once(w: &mut Acceptance) {
    let mut baseline = w.result["baseline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect::<Vec<_>>();
    baseline.sort_unstable();
    for result in [&w.result, &w.replay] {
        let mut names = result["runs"][0]["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(names, baseline);
    }
}
#[then("only selected expanded cases appear in the replayed permutation")]
fn selected_plan(w: &mut Acceptance) {
    assert_eq!(
        w.result["baseline"],
        json!(["first", "row one", "row three", "final"])
    );
    assert_eq!(w.result["records"][0]["count"], 4);
    assert_eq!(w.replay["baseline"], w.result["baseline"]);
    exactly_once(w);
}
#[then("the sink receives exactly one report matching the stored realization")]
fn custom_report(w: &mut Acceptance) {
    let timeline = w.result["timeline"].as_array().unwrap();
    let reports: Vec<_> =
        timeline.iter().filter(|e| e["kind"] == "report").collect();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0]["record"], w.result["records"][0]);
    assert_eq!(timeline[0]["kind"], "report");
    valid_seed(seed(&w.result));
}
#[then("the default generated-seed stderr report is suppressed")]
#[then("no generated-seed report is emitted")]
fn no_stderr(w: &mut Acceptance) {
    assert!(!w.stderr.contains("Random order using seed:"));
    if w.mode != "custom" {
        assert!(
            !w.result["timeline"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "report")
        );
    }
}
#[then("there is one distinct run record for each actual permutation")]
fn reuse_records(w: &mut Acceptance) {
    let records = w.result["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_ne!(records[0]["run_id"], records[1]["run_id"]);
    assert_eq!(w.result["runs"].as_array().unwrap().len(), 2);
    for record in records {
        valid_seed(record["seed"].as_str().unwrap());
        assert_eq!(record["order"], "generated");
    }
}
#[then("each run reports exactly its own recorded decimal seed")]
fn own_reports(w: &mut Acceptance) {
    let timeline = w.result["timeline"].as_array().unwrap();
    let reports: Vec<_> =
        timeline.iter().filter(|e| e["kind"] == "report").collect();
    assert_eq!(reports.len(), 2);
    for (index, report) in reports.iter().enumerate() {
        let record = &w.result["records"][index];
        assert_eq!(report["record"], *record);
        let reported = timeline
            .iter()
            .position(|e| {
                e["kind"] == "report"
                    && e["record"]["run_id"] == record["run_id"]
            })
            .unwrap();
        let first_start = timeline
            .iter()
            .position(|e| {
                e["kind"] == "start" && e["run_id"] == record["run_id"]
            })
            .unwrap();
        assert!(reported < first_start);
    }
    assert!(!w.stderr.contains("Random order using seed:"));
}
#[then("replay is verified without requiring the two random seeds to differ")]
fn reuse_replay(w: &mut Acceptance) {
    for index in 0..2 {
        let seed = w.result["records"][index]["seed"].as_str().unwrap();
        let expected = oracle(&w.result["baseline"], seed);
        let (replayed, stderr) = invoke("replay", Some(seed));
        assert_eq!(w.result["runs"][index]["names"], expected);
        assert_eq!(replayed["runs"][0]["names"], expected);
        assert!(!stderr.contains("Random order using seed:"));
    }
}
#[then("its order matches the pinned JavaScript empty-seed shuffle")]
fn empty_seed(w: &mut Acceptance) {
    assert_eq!(seed(&w.result), "");
    assert_eq!(w.result["records"][0]["order"], "explicit");
    assert_eq!(w.result["runs"][0]["names"], oracle(&w.result["baseline"], ""));
}
#[then(
    expr = "the selected plan follows its existing {string} ordering contract"
)]
fn existing(w: &mut Acceptance, order: String) {
    let mut expected = w.result["baseline"].as_array().unwrap().clone();
    if order == "reverse" {
        expected.reverse();
    }
    assert_eq!(w.result["records"].as_array().unwrap().len(), 1);
    assert_eq!(w.result["records"][0]["order"], order);
    assert!(w.result["records"][0]["seed"].is_null());
    assert_eq!(w.result["runs"][0]["names"], json!(expected));
}
#[then("the unused ordering store remains empty")]
fn unused(w: &mut Acceptance) {
    assert!(w.result["records"].as_array().unwrap().is_empty());
}
#[then("no application World or case callback is constructed or invoked")]
fn no_world(w: &mut Acceptance) {
    assert_eq!(w.result["runs"][0]["worlds"], 0);
    assert_eq!(w.result["runs"][0]["passed"], 0);
    assert!(w.result["runs"][0]["names"].as_array().unwrap().is_empty());
    assert!(w.result["timeline"].as_array().unwrap().is_empty());
    assert!(!w.stderr.contains("CASE_START:"));
}
#[then("one generated ordering record describes zero selected cases")]
fn zero_record(w: &mut Acceptance) {
    generated_record(w);
    assert_eq!(w.result["records"][0]["count"], 0);
    assert!(w.result["baseline"].as_array().unwrap().is_empty());
}
#[then("the empty plan reports its recorded seed exactly once")]
fn zero_report(w: &mut Acceptance) {
    let expected = format!("Random order using seed: {}", seed(&w.result));
    assert_eq!(w.stderr.matches("Random order using seed:").count(), 1);
    assert!(w.stderr.contains(&expected));
}
#[tokio::main]
async fn main() {
    if let Ok(request) = std::env::var("CUCUMBER_GENERATED_ORDER_CHILD") {
        println!("{}", child(serde_json::from_str(&request).unwrap()).await);
        return;
    }
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/generated_ordering")
        .await;
}
