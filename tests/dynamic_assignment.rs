//! Phase37b actual assignment, resource reservations, idle diagnostics and panic propagation.
use cucumber::{
    Event, StatsWriter as _, World as _, WriterExt as _, event, given,
    runner::{
        AssignmentCase,
        AssignmentDecisionKind as Decision, AssignmentPolicy, AssignmentStore,
        Basic, Hooks, ScenarioOrder, ScenarioType, SchedulingOptions,
        TimeoutClock,
    },
    step::{Collection, Context},
    then, when,
    writer::{self, Coloring},
};
use futures::{
    FutureExt as _,
    future::{BoxFuture, LocalBoxFuture},
};
use regex::Regex;
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, HashMap},
    convert::Infallible,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Barrier, Semaphore};
#[derive(Debug, Default)]
struct Seen {
    starts: Vec<String>,
    finishes: Vec<String>,
    bodies: Vec<String>,
    identities: Vec<(String, usize, usize, String)>,
    active: HashMap<String, (BTreeSet<String>, bool)>,
    peak: usize,
    overlap: bool,
    finished: bool,
    midpoint: bool,
    bounded: bool,
}
#[derive(Debug)]
struct Gates {
    pair: Barrier,
    entered: Semaphore,
    release: Semaphore,
    delay_entered: Semaphore,
    delay_release: Semaphore,
    c_entered: Semaphore,
    c_release: Semaphore,
    c_finished: Semaphore,
}
impl Default for Gates {
    fn default() -> Self {
        Self {
            pair: Barrier::new(2),
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
            delay_entered: Semaphore::new(0),
            delay_release: Semaphore::new(0),
            c_entered: Semaphore::new(0),
            c_release: Semaphore::new(0),
            c_finished: Semaphore::new(0),
        }
    }
}
async fn permit(s: &Semaphore) {
    s.acquire().await.unwrap().forget();
}
#[derive(Debug)]
struct Clock(Arc<Gates>);
impl TimeoutClock for Clock {
    fn sleep(&self, _: Duration) -> BoxFuture<'static, ()> {
        let gates = self.0.clone();
        Box::pin(async move {
            gates.delay_entered.add_permits(1);
            permit(&gates.delay_release).await;
        })
    }
}
#[derive(Debug, Default)]
struct Application {
    name: String,
    attempt: usize,
    id: usize,
    mode: String,
    seen: Option<Arc<Mutex<Seen>>>,
    gates: Option<Arc<Gates>>,
}
impl cucumber::World for Application {
    type Error = Infallible;
    async fn new() -> Result<Self, Self::Error> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Ok(Self { id: NEXT.fetch_add(1, Ordering::SeqCst), ..Self::default() })
    }
}
fn resources(
    feature: &gherkin::Feature,
    rule: Option<&gherkin::Rule>,
    scenario: &gherkin::Scenario,
) -> BTreeSet<String> {
    feature
        .tags
        .iter()
        .chain(rule.into_iter().flat_map(|r| &r.tags))
        .chain(&scenario.tags)
        .filter(|tag| tag.starts_with("resource."))
        .cloned()
        .collect()
}
fn body(w: &mut Application, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        let seen = w.seen.as_ref().unwrap();
        let gates = w.gates.as_ref().unwrap();
        seen.lock().unwrap().bodies.push(format!("{}:{}", w.name, w.attempt));
        if matches!(
            w.mode.as_str(),
            "shared browser" | "first eligible" | "builder preservation"
        ) && matches!(w.name.as_str(), "A" | "C")
        {
            gates.pair.wait().await;
        }
        if w.mode == "independent resources" {
            gates.pair.wait().await;
        }
        if matches!(
            w.mode.as_str(),
            "wait for release" | "queued Serial barrier" | "active Serial"
        ) && w.name == "A"
        {
            gates.entered.add_permits(1);
            permit(&gates.release).await;
        }
        if w.mode == "retry delay" {
            if w.name == "C" {
                gates.c_entered.add_permits(1);
                permit(&gates.c_release).await;
            }
            assert!(
                !(w.name == "A" && w.attempt == 0),
                "actual retrying resource owner"
            );
        }
        tokio::task::yield_now().await;
    })
}
#[derive(Debug, Clone)]
struct Recorder(Arc<Mutex<Seen>>);
impl writer::NonTransforming for Recorder {}
impl cucumber::Writer<Application> for Recorder {
    type Cli = cucumber::cli::Empty;
    async fn handle_event(
        &mut self,
        input: cucumber::parser::Result<Event<event::Cucumber<Application>>>,
        _: &Self::Cli,
    ) {
        if matches!(input.unwrap().value, event::Cucumber::Finished) {
            self.0.lock().unwrap().finished = true;
        }
    }
}
fn source(mode: &str) -> String {
    let case = |name: &str, key: &str, serial: bool| {
        format!(
            " @resource.{key}{}\n Scenario: {name}\n  Given work\n",
            if serial { " @serial" } else { "" }
        )
    };
    let mut source = "Feature: Assignment resources\n".to_owned();
    match mode{
        "outline resource tags"=>return "@resource.workspace-X\nFeature: Assignment resources\n @rule\n Rule: Browser resource\n  Scenario Outline: row <id>\n   Given work\n   @resource.browser-X\n   Examples: selected\n    | id |\n    | one |\n    | two |\n".into(),
        "independent resources"=>{source+=&case("A","browser-X",false);source+=&case("B","browser-Y",false);},
        "queued Serial barrier"=>{source+=&case("A","browser-X",false);source+=&case("S","browser-Y",true);source+=&case("C","browser-Z",false);},
        "active Serial"=>{source+=&case("A","browser-X",true);source+=&case("B","browser-Y",false);source+=&case("C","browser-Z",false);},
        "rejected Serial"=>{source+=&case("S","browser-X",true);source+=&case("B","browser-Y",false);},
        "default native"=>{source+=&case("A","browser-X",false);source+=&case("S","browser-Y",true);source+=&case("B","browser-Z",false);},
        _=>{source+=&case("A","browser-X",false);source+=&case("B","browser-X",false);if mode!="wait for release"{source+=&case("C","browser-Y",false);}},
    }
    source
}
fn case_json(case: &AssignmentCase) -> Value {
    json!({"id":case.case_id,"index":case.plan_index,"name":case.scenario.name,"path":case.feature.path,"line":case.scenario.position.line,"rule":case.rule.as_ref().map(|r|r.name.clone()),"resources":resources(&case.feature,case.rule.as_deref(),&case.scenario),"serial":case.kind==ScenarioType::Serial})
}
fn records(store: &AssignmentStore) -> Vec<Value> {
    store.records().into_iter().map(|r|{
    let content=match r.kind{
        Decision::Evaluated{candidate,running,eligible}=>json!({"kind":"evaluated","case":case_json(&candidate),"running":running.iter().map(case_json).collect::<Vec<_>>(),"eligible":eligible}),
        Decision::Admitted{case,reason}=>json!({"kind":"admitted","case":case_json(&case),"reason":format!("{reason:?}")}),
        Decision::Released{case_id}=>json!({"kind":"released","id":case_id}),
        Decision::Waiting{reason,running}=>json!({"kind":"waiting","reason":format!("{reason:?}"),"running":running.iter().map(case_json).collect::<Vec<_>>()}),
        Decision::Warning(warning)=>json!({"kind":"warning","count":warning.idle_interventions,"message":warning.message}),
        _=>panic!("unexpected assignment decision"),
    };json!({"run":r.run_id,"sequence":r.sequence,"decision":content})
}).collect()
}
async fn execute(mode: String) -> Value {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let gates = Arc::new(Gates::default());
    let store = AssignmentStore::new();
    let warnings = Arc::new(Mutex::new(Vec::<String>::new()));
    let predicate_mode = mode.clone();
    let mut policy = AssignmentPolicy::new(
        "resource eligibility",
        move |candidate, running| {
            assert_ne!(
                predicate_mode, "predicate panic",
                "original predicate programming error"
            );
            if matches!(
                predicate_mode.as_str(),
                "idle fallback" | "custom warning sink"
            ) {
                return false;
            }
            if predicate_mode == "rejected Serial" {
                return candidate.kind != ScenarioType::Serial;
            }
            if matches!(
                predicate_mode.as_str(),
                "queued Serial barrier" | "active Serial"
            ) {
                return true;
            }
            let requested = resources(
                &candidate.feature,
                candidate.rule.as_deref(),
                &candidate.scenario,
            );
            running.iter().all(|active| {
                requested.is_disjoint(&resources(
                    &active.feature,
                    active.rule.as_deref(),
                    &active.scenario,
                ))
            })
        },
    )
    .with_store(store.clone());
    if mode != "idle fallback" {
        let warnings = warnings.clone();
        policy = policy.on_warning(move |warning| {
            warnings.lock().unwrap().push(warning.message.clone())
        });
    }
    let before = seen.clone();
    let after = seen.clone();
    let bgates = gates.clone();
    let agates = gates.clone();
    let config = mode.clone();
    let hooks = Hooks::new()
        .before_scoped(
            "record actual start",
            move |scope, w: &mut Application| {
                w.name = scope.attempt.scenario.name.clone();
                w.attempt = scope.attempt.attempt;
                w.mode = config.clone();
                w.seen = Some(before.clone());
                w.gates = Some(bgates.clone());
                let keys = resources(
                    &scope.attempt.feature,
                    scope.attempt.rule.as_deref(),
                    &scope.attempt.scenario,
                );
                let serial = scope
                    .attempt
                    .scenario
                    .tags
                    .iter()
                    .any(|tag| tag == "serial");
                let mut state = before.lock().unwrap();
                if state.active.values().any(|(other, active_serial)| {
                    serial || *active_serial || !keys.is_disjoint(other)
                }) {
                    state.overlap = true;
                }
                state.active.insert(w.name.clone(), (keys, serial));
                state.peak = state.peak.max(state.active.len());
                state.starts.push(format!("{}:{}", w.name, w.attempt));
                state.identities.push((
                    w.name.clone(),
                    w.attempt,
                    w.id,
                    format!("{:?}", scope.attempt.scenario_id),
                ));
                Box::pin(async {})
            },
        )
        .after_scoped("record actual finish", move |_, w| {
            let after = after.clone();
            let gates = agates.clone();
            Box::pin(async move {
                let w = w.unwrap();
                let mut state = after.lock().unwrap();
                state.active.remove(&w.name);
                state.finishes.push(format!("{}:{}", w.name, w.attempt));
                if w.mode == "retry delay" && w.name == "C" {
                    gates.c_finished.add_permits(1);
                }
            })
        });
    let mut options = SchedulingOptions::new(ScenarioOrder::Defined)
        .parallel(2)
        .with_assignment(policy);
    if mode == "retry delay" {
        options = options.with_retry_clock(Arc::new(Clock(gates.clone())));
    }
    let base = Basic::<Application>::default();
    let base = if mode == "default native" {
        base
    } else {
        base.with_scheduling_options(options.clone())
    };
    let base = base.clone().which_scenario(|_, _, scenario| {
        if scenario.tags.iter().any(|t| t == "serial") {
            ScenarioType::Serial
        } else {
            ScenarioType::Concurrent
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("assignment.feature");
    std::fs::write(&path, source(&mode)).unwrap();
    let writer = writer::Basic::raw(std::io::sink(), Coloring::Never, 0)
        .tee::<Application, _>(
            Recorder(seen.clone()).discard_arbitrary_writes(),
        )
        .normalized()
        .summarized();
    let mut app = cucumber::Cucumber::new()
        .with_runner(base)
        .steps(Collection::new().given(
            None,
            Regex::new("^work$").unwrap(),
            body,
        ))
        .with_hooks(hooks)
        .max_concurrent_scenarios(2)
        .with_writer(writer);
    if mode == "retry delay" {
        app = app.retries(1).retry_after(Duration::from_secs(123));
    }
    let running =
        std::panic::AssertUnwindSafe(app.with_default_cli().run(&path))
            .catch_unwind();
    let driver = async {
        if matches!(
            mode.as_str(),
            "wait for release" | "queued Serial barrier" | "active Serial"
        ) {
            permit(&gates.entered).await;
            for _ in 0..16 {
                tokio::task::yield_now().await;
            }
            assert_eq!(seen.lock().unwrap().starts, vec!["A:0"]);
            let count = store.records().len();
            for _ in 0..32 {
                tokio::task::yield_now().await;
            }
            assert_eq!(
                store.records().len(),
                count,
                "blocked dispatcher must not repeatedly evaluate the same reservation set"
            );
            seen.lock().unwrap().bounded = true;
            seen.lock().unwrap().midpoint = true;
            gates.release.add_permits(1);
        }
        if mode == "retry delay" {
            permit(&gates.delay_entered).await;
            permit(&gates.c_entered).await;
            assert!(
                !seen
                    .lock()
                    .unwrap()
                    .starts
                    .iter()
                    .any(|s| s.starts_with("B:"))
            );
            gates.c_release.add_permits(1);
            permit(&gates.c_finished).await;
            for _ in 0..32 {
                tokio::task::yield_now().await;
            }
            assert!(store.records().iter().any(|r|matches!(&r.kind,Decision::Waiting{running,..}if running.iter().any(|c|c.scenario.name=="A"))));
            assert!(
                !seen
                    .lock()
                    .unwrap()
                    .starts
                    .iter()
                    .any(|s| s.starts_with("B:"))
            );
            seen.lock().unwrap().midpoint = true;
            gates.delay_release.add_permits(1);
        }
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(running, driver)
    })
    .await
    .expect("assignment must progress under its controlled gates");
    let (passed, failed, panic) = match result {
        Ok(report) => (report.passed_steps(), report.failed_steps(), None),
        Err(error) => {
            let message = error
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    error.downcast_ref::<&str>().map(|s| (*s).to_owned())
                })
                .unwrap_or_else(|| "unknown panic".into());
            (0, 0, Some(message))
        }
    };
    let state = seen.lock().unwrap();
    json!({"records":records(&store),"warnings":warnings.lock().unwrap().clone(),"current_run":store.current_run(),"starts":state.starts,"finishes":state.finishes,"bodies":state.bodies,"identities":state.identities,"peak":state.peak,"overlap":state.overlap,"finished":state.finished,"midpoint":state.midpoint,"bounded":state.bounded,"passed":passed,"failed":failed,"panic":panic})
}
#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    mode: String,
    result: Value,
    stderr: String,
}
#[given(expr = "ordered cases configured for {string}")]
fn configured(w: &mut Acceptance, mode: String) {
    w.mode = mode;
}
#[when("the application runs with explicit dynamic assignment")]
#[when("the application runs without dynamic assignment")]
#[when("the application catches its configured predicate failure")]
fn run(w: &mut Acceptance) {
    let result = Command::new(std::env::current_exe().unwrap())
        .env("CUCUMBER_ASSIGNMENT_CHILD", &w.mode)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "child failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    w.stderr = String::from_utf8(result.stderr).unwrap();
    w.result = serde_json::from_slice(&result.stdout).unwrap();
    if w.mode != "predicate panic" {
        assert!(w.result["panic"].is_null(), "{}", w.result);
        assert_eq!(w.result["failed"], 0);
        assert_eq!(w.result["finished"], true);
    }
}
fn decisions(w: &Acceptance) -> Vec<&Value> {
    w.result["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| &r["decision"])
        .collect()
}
fn admissions(w: &Acceptance) -> Vec<&Value> {
    decisions(w).into_iter().filter(|r| r["kind"] == "admitted").collect()
}
fn names(w: &Acceptance) -> Vec<&str> {
    admissions(w).into_iter().map(|r| r["case"]["name"].as_str().unwrap()).collect()
}
#[then("cases sharing a browser resource execute exclusively")]
fn exclusive(w: &mut Acceptance) {
    assert_eq!(w.result["overlap"], false);
    assert_eq!(w.result["passed"], 3);
    assert_eq!(names(w), vec!["A", "C", "B"]);
}
#[then(
    "observed decisions refer to actual logical cases rather than worker identities"
)]
fn identities(w: &mut Acceptance) {
    let records = w.result["records"].as_array().unwrap();
    let run = records[0]["run"].clone();
    for (index, r) in records.iter().enumerate() {
        assert_eq!(r["run"], run);
        assert_eq!(r["sequence"], index);
    }
    let mut ids = BTreeSet::new();
    for a in admissions(w) {
        assert!(ids.insert(a["case"]["id"].as_u64().unwrap()));
        assert!(
            a["case"]["path"].as_str().unwrap().ends_with("assignment.feature")
        );
        assert!(a["case"]["line"].as_u64().unwrap() > 0);
    }
    assert_eq!(
        decisions(w).iter().filter(|r| r["kind"] == "released").count(),
        3
    );
    assert!(!w.result.to_string().contains("workerId"));
}
#[then("independent cases overlap within the configured capacity")]
fn concurrent(w: &mut Acceptance) {
    assert_eq!(w.result["peak"], 2);
    assert_eq!(w.result["overlap"], false);
    assert_eq!(w.result["passed"], 2);
}
#[then("assignment callbacks see the previously admitted logical reservations")]
fn snapshots(w: &mut Acceptance) {
    let rows = decisions(w);
    let b = rows
        .iter()
        .find(|r| r["kind"] == "evaluated" && r["case"]["name"] == "B")
        .unwrap();
    assert_eq!(b["running"][0]["name"], "A");
}
#[then("the first eligible candidate is admitted in queue order")]
fn first_eligible(w: &mut Acceptance) {
    assert_eq!(names(w), vec!["A", "C", "B"]);
}
#[then("an earlier rejected case remains queued for later consideration")]
fn later(w: &mut Acceptance) {
    let b: Vec<_> = decisions(w)
        .into_iter()
        .filter(|r| r["kind"] == "evaluated" && r["case"]["name"] == "B")
        .collect();
    assert_eq!(b[0]["eligible"], false);
    assert_eq!(b.last().unwrap()["eligible"], true);
    assert_eq!(b[0]["case"]["index"], 1);
    assert_eq!(b.last().unwrap()["case"]["index"], 1);
}
#[then(
    "blocked cases remain unassigned until the active case releases its reservation"
)]
fn waiting(w: &mut Acceptance) {
    assert_eq!(w.result["midpoint"], true);
    assert_eq!(names(w), vec!["A", "B"]);
    assert_eq!(w.result["overlap"], false);
}
#[then(
    "the dispatcher waits without repeated idle decisions or unbounded record growth"
)]
fn bounded(w: &mut Acceptance) {
    assert_eq!(w.result["bounded"], true);
    assert_eq!(
        decisions(w).iter().filter(|r| r["kind"] == "waiting").count(),
        1
    );
    assert!(w.result["records"].as_array().unwrap().len() < 12);
}
#[then("the retrying logical case remains assigned throughout its delay")]
fn delayed(w: &mut Acceptance) {
    assert_eq!(w.result["midpoint"], true);
    let rows = decisions(w);
    assert!(rows.iter().any(|r| r["kind"] == "waiting"
        && r["running"].as_array().unwrap().iter().any(|c| c["name"] == "A")));
    assert_eq!(names(w), vec!["A", "C", "B"]);
}
#[then("its fresh retry World does not create a second logical admission")]
fn retry_identity(w: &mut Acceptance) {
    assert_eq!(
        admissions(w).iter().filter(|r| r["case"]["name"] == "A").count(),
        1
    );
    let actual: Vec<_> = w.result["identities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r[0] == "A")
        .collect();
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0][1], 0);
    assert_eq!(actual[1][1], 1);
    assert_ne!(actual[0][2], actual[1][2]);
    assert_ne!(actual[0][3], actual[1][3]);
}
#[then(
    "independent resources can proceed while conflicting resources remain blocked"
)]
fn delayed_parallel(w: &mut Acceptance) {
    let starts = w.result["starts"].as_array().unwrap();
    let c = starts.iter().position(|s| s == "C:0").unwrap();
    let a = starts.iter().position(|s| s == "A:1").unwrap();
    let b = starts.iter().position(|s| s == "B:0").unwrap();
    assert!(c < a && a < b);
    assert_eq!(w.result["overlap"], false);
}
#[then(
    "no candidate after the first Serial barrier is evaluated or admitted early"
)]
fn serial_prefix(w: &mut Acceptance) {
    let rows = decisions(w);
    let sid = admissions(w).iter().find(|a| a["case"]["name"] == "S").unwrap()
        ["case"]["id"]
        .clone();
    let release = rows
        .iter()
        .position(|r| r["kind"] == "released" && r["id"] == sid)
        .unwrap();
    let c = rows
        .iter()
        .position(|r| r["kind"] == "evaluated" && r["case"]["name"] == "C")
        .unwrap();
    assert!(release < c);
    assert_eq!(names(w), vec!["A", "S", "C"]);
}
#[then(
    "the Serial case starts only after all preceding assigned cases complete"
)]
fn serial_after(w: &mut Acceptance) {
    assert_eq!(w.result["midpoint"], true);
    assert_eq!(w.result["overlap"], false);
}
#[then("the Serial case never overlaps another logical reservation")]
fn serial_exclusive(w: &mut Acceptance) {
    assert_eq!(w.result["overlap"], false);
    let rows = decisions(w);
    let release = rows
        .iter()
        .position(|r| r["kind"] == "released" && r["id"] == 0)
        .unwrap();
    let next = rows
        .iter()
        .position(|r| r["kind"] == "admitted" && r["case"]["name"] == "B")
        .unwrap();
    assert!(release < next);
}
#[then("idle fallback chooses the Serial front and preserves its exclusivity")]
fn serial_fallback(w: &mut Acceptance) {
    let a = admissions(w);
    assert_eq!(a[0]["case"]["name"], "S");
    assert_eq!(a[0]["reason"], "IdleFallback");
    assert_eq!(w.result["overlap"], false);
    assert_eq!(w.result["passed"], 2);
}
#[then(
    "each idle fallback admits the queue head without claiming predicate eligibility"
)]
fn fallback(w: &mut Acceptance) {
    assert_eq!(names(w), vec!["A", "B", "C"]);
    assert!(admissions(w).iter().all(|a| a["reason"] == "IdleFallback"));
    assert!(
        decisions(w)
            .iter()
            .filter(|r| r["kind"] == "evaluated")
            .all(|r| r["eligible"] == false)
    );
}
#[then(
    "a single aggregate warning reports the actual idle intervention count on stderr"
)]
fn stderr_warning(w: &mut Acceptance) {
    assert_eq!(w.stderr.matches("WARNING:").count(), 1);
    assert!(w.stderr.contains("3 idle fallback(s)"));
    assert!(w.result["warnings"].as_array().unwrap().is_empty());
}
#[then(
    "skipped predicate choices are not fabricated callback or worker execution records"
)]
fn honest(w: &mut Acceptance) {
    assert_eq!(w.result["bodies"].as_array().unwrap().len(), 3);
    assert!(!w.result["records"].to_string().contains("invocation_id"));
    assert!(!w.result["records"].to_string().contains("worker"));
}
#[then("the configured sink receives one typed aggregate warning")]
fn sink(w: &mut Acceptance) {
    let warnings = w.result["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].as_str().unwrap().contains("3 idle fallback(s)"));
}
#[then("the same warning is retained in the assignment decision store")]
fn stored_warning(w: &mut Acceptance) {
    let records: Vec<_> =
        decisions(w).into_iter().filter(|r| r["kind"] == "warning").collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["count"], 3);
    assert_eq!(records[0]["message"], w.result["warnings"][0]);
}
#[then("the default stderr warning is suppressed")]
fn no_stderr(w: &mut Acceptance) {
    assert!(!w.stderr.contains("WARNING:"), "{}", w.stderr);
}
#[then(
    "the predicate observes expanded native source identities and inherited resource tags"
)]
fn outline(w: &mut Acceptance) {
    assert_eq!(names(w), vec!["row one", "row two"]);
    for a in admissions(w) {
        assert_eq!(a["case"]["rule"], "Browser resource");
        assert_eq!(
            a["case"]["resources"],
            json!(["resource.browser-X", "resource.workspace-X"])
        );
        assert!(
            a["case"]["path"].as_str().unwrap().ends_with("assignment.feature")
        );
    }
}
#[then("decision snapshots preserve their fixed ordered-plan positions")]
fn positions(w: &mut Acceptance) {
    for (index, a) in admissions(w).iter().enumerate() {
        assert_eq!(a["case"]["index"], index);
        for evaluation in decisions(w).into_iter().filter(|r| {
            r["kind"] == "evaluated" && r["case"]["id"] == a["case"]["id"]
        }) {
            assert_eq!(evaluation["case"]["index"], index);
        }
    }
}
#[then("the installed policy and shared decision store remain active")]
fn retained(w: &mut Acceptance) {
    assert!(!w.result["current_run"].is_null());
    assert_eq!(names(w), vec!["A", "C", "B"]);
    assert_eq!(w.result["overlap"], false);
}
#[then(
    "ordinary native scheduling executes without assignment callbacks or warnings"
)]
fn native(w: &mut Acceptance) {
    assert!(w.result["records"].as_array().unwrap().is_empty());
    assert!(w.result["current_run"].is_null());
    assert!(w.result["warnings"].as_array().unwrap().is_empty());
    assert!(!w.stderr.contains("WARNING:"));
    assert_eq!(w.result["passed"], 3);
    assert_eq!(w.result["starts"][0], "S:0");
}
#[then(
    "the original panic propagates and no candidate is admitted by fallback"
)]
fn panic(w: &mut Acceptance) {
    assert!(
        w.result["panic"]
            .as_str()
            .unwrap()
            .contains("original predicate programming error")
    );
    assert!(admissions(w).is_empty());
    assert!(w.result["starts"].as_array().unwrap().is_empty());
}
#[then("no completed run or successful callback is invented")]
fn incomplete(w: &mut Acceptance) {
    assert_eq!(w.result["finished"], false);
    assert_eq!(w.result["passed"], 0);
    assert!(w.result["warnings"].as_array().unwrap().is_empty());
}
#[tokio::main]
async fn main() {
    if let Ok(mode) = std::env::var("CUCUMBER_ASSIGNMENT_CHILD") {
        println!("{}", execute(mode).await);
        return;
    }
    Acceptance::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/dynamic_assignment")
        .await;
}
