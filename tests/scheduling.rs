//! Phase 5 acceptance of the native async scheduler, not process workers.
use std::{collections::BTreeSet, sync::{Arc, Mutex}, time::Duration};

use cucumber::{StatsWriter as _, World as _, WriterExt as _, given, then, when,
    writer::{self, Coloring}};
use tokio::sync::Barrier;

#[derive(Debug, Default)]
struct Observed {
    active: usize,
    peak: usize,
    serial_active: usize,
    serial_runs: usize,
    started: Vec<String>,
    completed: Vec<(String, Vec<String>)>,
    violations: Vec<String>,
    step_entries: usize,
}

#[derive(Debug, Default, cucumber::World)]
struct Application {
    observed: Option<Arc<Mutex<Observed>>>,
    barrier: Option<Arc<Barrier>>,
    identity: String,
    ledger: Vec<String>,
    serial: bool,
    fail: bool,
}

#[given("perform isolated work")]
async fn work(world: &mut Application) {
    world.observed.as_ref().unwrap().lock().unwrap().step_entries += 1;
    assert!(!world.fail, "intentional application failure for fail-fast acceptance");
    assert!(world.ledger.is_empty(), "each scenario must start with fresh state");
    world.ledger.push(world.identity.clone());
    if world.serial {
        // Give another runnable future an opportunity to overlap if the runner
        // incorrectly schedules it, without assuming any wall-clock duration.
        tokio::task::yield_now().await;
    } else {
        // All members of a cohort must be active before any can leave this step.
        // A sequential implementation cannot satisfy a barrier of size > 1.
        world.barrier.as_ref().unwrap().wait().await;
        tokio::task::yield_now().await;
    }
    assert_eq!(world.ledger, vec![world.identity.clone()], "World values must survive overlap unchanged");
}

#[derive(Debug, Default, cucumber::World)]
struct Scheduling {
    source: Vec<String>,
    count: usize,
    limit: Option<usize>,
    observed: Arc<Mutex<Observed>>,
    scenarios: (usize, usize, usize),
    steps: (usize, usize, usize),
    output: String,
    fail: bool,
}

fn scenario(name: &str) -> String {
    format!("  Scenario: {name}\n    Given perform isolated work\n")
}

#[given(regex = r"^(\d+) scenarios with independent World state$")]
fn independent(world: &mut Scheduling, count: usize) {
    world.count = count;
    let mut source = "Feature: Independent accounts\n".to_owned();
    for index in 0..count {
        source.push_str(&scenario(&format!("account-{index}")));
    }
    world.source = vec![source];
}

#[given(regex = r#"^concurrent scenarios and serial scenarios tagged at "([^"]+)" scope$"#)]
fn mixed(world: &mut Scheduling, scope: String) {
    world.count = 6;
    let mut concurrent = "Feature: Shared capacity\n".to_owned();
    for index in 0..4 {
        concurrent.push_str(&scenario(&format!("concurrent-{index}")));
    }
    match scope.as_str() {
        "scenario" => {
            for index in 0..2 {
                concurrent.push_str("  @serial\n");
                concurrent.push_str(&scenario(&format!("serial-{index}")));
            }
            world.source = vec![concurrent];
        }
        "rule" => {
            concurrent.push_str("  @serial\n  Rule: Exclusive shared resource\n");
            for index in 0..2 {
                concurrent.push_str(&scenario(&format!("serial-{index}")));
            }
            world.source = vec![concurrent];
        }
        "feature" => {
            let mut serial = "@serial\nFeature: Exclusive shared resource\n".to_owned();
            for index in 0..2 {
                serial.push_str(&scenario(&format!("serial-{index}")));
            }
            world.source = vec![serial, concurrent];
        }
        _ => panic!("unsupported scope: {scope}"),
    }
}

#[given(regex = r#"^a concurrency limit of "([^"]+)"$"#)]
fn configure_limit(world: &mut Scheduling, limit: String) {
    world.limit = if limit == "unlimited" { None } else { Some(limit.parse().unwrap()) };
}

#[given(regex = r"^(\d+) scenarios whose first step fails$")]
fn failures(world: &mut Scheduling, count: usize) {
    independent(world, count);
    world.fail = true;
}

async fn execute(world: &mut Scheduling, fail_fast: bool) {
    let directory = tempfile::tempdir().unwrap();
    for (index, source) in world.source.iter().enumerate() {
        std::fs::write(directory.path().join(format!("{index}.feature")), source).unwrap();
    }
    let shared = Arc::clone(&world.observed);
    let finished = Arc::clone(&world.observed);
    let barrier = Arc::new(Barrier::new(world.limit.unwrap_or(world.count)));
    let fail = world.fail;
    let mut output = Vec::new();
    let runner = Application::cucumber()
        .max_concurrent_scenarios(world.limit)
        .before(move |feature, rule, scenario, application| {
            let shared = Arc::clone(&shared);
            let barrier = Arc::clone(&barrier);
            Box::pin(async move {
                assert!(application.ledger.is_empty(), "World reused before setup");
                application.identity = scenario.name.clone();
                application.serial = scenario.tags.iter()
                    .chain(rule.iter().flat_map(|rule| &rule.tags))
                    .chain(&feature.tags).any(|tag| tag == "serial");
                application.fail = fail;
                application.observed = Some(Arc::clone(&shared));
                application.barrier = Some(barrier);
                let mut observed = shared.lock().unwrap();
                if application.serial {
                    if observed.active != 0 {
                        observed.violations.push("serial scenario started during other work".into());
                    }
                    observed.serial_active += 1;
                    observed.serial_runs += 1;
                } else if observed.serial_active != 0 {
                    observed.violations.push("concurrent scenario started during serial work".into());
                }
                observed.active += 1;
                observed.peak = observed.peak.max(observed.active);
                observed.started.push(application.identity.clone());
            })
        })
        .after(move |_, _, _, _, application| {
            let finished = Arc::clone(&finished);
            Box::pin(async move {
                let application = application.expect("World reaches teardown");
                let mut observed = finished.lock().unwrap();
                observed.active -= 1;
                if application.serial { observed.serial_active -= 1; }
                observed.completed.push((application.identity.clone(), application.ledger.clone()));
            })
        })
        .with_writer(writer::Basic::new(&mut output, Coloring::Never, 0).summarized())
        .with_default_cli();
    let runner = if fail_fast { runner.fail_fast() } else { runner };
    let report = tokio::time::timeout(Duration::from_secs(15), runner.run(directory.path()))
        .await.expect("scheduler deadlocked at a deterministic cohort barrier");
    assert_eq!(report.parsing_errors(), 0, "inner features must parse");
    assert_eq!(report.hook_errors(), 0, "observation hooks must succeed");
    let stats = report.scenarios_stats();
    world.scenarios = (stats.passed, stats.failed, stats.skipped);
    world.steps = (report.passed_steps(), report.failed_steps(), report.skipped_steps());
    drop(report);
    world.output = String::from_utf8(output).unwrap();
}

#[when("the coordinated feature is executed")]
async fn run(world: &mut Scheduling) { execute(world, false).await; }

#[when("the coordinated feature is executed with fail fast")]
async fn run_fail_fast(world: &mut Scheduling) { execute(world, true).await; }

#[then(regex = r"^all (\d+) scenarios pass with isolated World values$")]
fn passed(world: &mut Scheduling, count: usize) {
    assert_eq!(world.scenarios, (count, 0, 0), "{}", world.output);
    assert_eq!(world.steps, (count, 0, 0), "{}", world.output);
    let observed = world.observed.lock().unwrap();
    assert_eq!(observed.started.len(), count);
    assert_eq!(observed.completed.len(), count);
    assert_eq!(observed.step_entries, count);
    let identities: BTreeSet<_> = observed.started.iter().collect();
    assert_eq!(identities.len(), count);
    for (identity, ledger) in &observed.completed {
        assert_eq!(ledger, &vec![identity.clone()]);
    }
    assert!(observed.violations.is_empty(), "{:?}", observed.violations);
}

#[then(regex = r"^the observed peak concurrency is (\d+)$")]
fn peak(world: &mut Scheduling, expected: usize) {
    assert_eq!(world.observed.lock().unwrap().peak, expected);
}

#[then("both serial scenarios ran exclusively")]
fn exclusive(world: &mut Scheduling) {
    let observed = world.observed.lock().unwrap();
    assert_eq!(observed.serial_runs, 2);
    assert!(observed.violations.is_empty(), "{:?}", observed.violations);
}

#[then("no scenarios remain active")]
fn inactive(world: &mut Scheduling) {
    let observed = world.observed.lock().unwrap();
    assert_eq!(observed.active, 0);
    assert_eq!(observed.serial_active, 0);
}

#[then("exactly one scenario fails and the other scenarios never start")]
fn stopped(world: &mut Scheduling) {
    assert_eq!(world.scenarios, (0, 1, 0), "{}", world.output);
    assert_eq!(world.steps, (0, 1, 0), "{}", world.output);
    let observed = world.observed.lock().unwrap();
    assert_eq!(observed.started.len(), 1);
    assert_eq!(observed.completed.len(), 1);
    assert_eq!(observed.step_entries, 1);
    assert!(observed.completed[0].1.is_empty());
    assert!(world.output.contains("intentional application failure"), "{}", world.output);
}

#[tokio::main]
async fn main() {
    Scheduling::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features/scheduling")
        .await;
}
