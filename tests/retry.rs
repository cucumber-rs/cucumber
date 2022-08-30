use std::collections::HashMap;

use cucumber::{gherkin::Step, given, writer::summarize::Stats, World as _};
use gherkin::tagexpr::TagOperation;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

static SCENARIO_RUNS: Lazy<Mutex<HashMap<Step, usize>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[given(expr = "fail {int} time(s)")]
async fn fail(_: &mut World, num: usize, step: &Step) {
    let mut guard = SCENARIO_RUNS.lock().await;
    let runs = guard.entry(step.clone()).or_default();
    *runs += 1;
    assert!(*runs > num);
}

#[tokio::main]
async fn main() {
    let op = |s: &str| Some(s.parse::<TagOperation>().unwrap());

    for ((p_sc, f_sc, r_sc, p_st, f_st, r_st), (retries, retry_filter)) in [
        ((0, 7, 4, 0, 7, 6), (None, None)),
        ((3, 4, 5, 3, 4, 13), (Some(5), op("@flaky"))),
        ((4, 3, 6, 4, 3, 16), (Some(5), op("@serial"))),
        ((5, 2, 7, 5, 2, 19), (Some(5), None)),
    ] {
        let writer = World::cucumber()
            .retries(retries)
            .retry_filter(retry_filter)
            .run("tests/features/retry")
            .await;

        assert_eq!(
            writer.scenarios,
            Stats {
                passed: p_sc,
                skipped: 0,
                failed: f_sc,
                retried: r_sc,
            },
            "Wrong Stats for Scenarios",
        );
        assert_eq!(
            writer.steps,
            Stats {
                passed: p_st,
                skipped: 0,
                failed: f_st,
                retried: r_st,
            },
            "Wrong Stats for Steps",
        );

        SCENARIO_RUNS.lock().await.clear();
    }
}

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World;
