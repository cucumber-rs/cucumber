use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use cucumber::{gherkin::Step, given, writer::summarize::Stats, World as _};
use futures::{stream, StreamExt as _};
use gherkin::tagexpr::TagOperation;
use once_cell::sync::Lazy;
use tokio::{sync::Mutex, time::sleep};

static SCENARIO_RUNS: Lazy<Mutex<HashMap<Step, usize>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[given(expr = "fail {int} time(s)")]
async fn fail(_: &mut World, num: usize, step: &Step) {
    sleep(Duration::from_secs(1)).await;

    let mut guard = SCENARIO_RUNS.lock().await;
    let runs = guard.entry(step.clone()).or_default();
    *runs += 1;
    assert!(*runs > num);
}

#[tokio::main]
async fn main() {
    let secs = |n| Duration::from_secs(n);
    let op = |s: &str| Some(s.parse::<TagOperation>().unwrap());

    let _ = stream::iter([
        ((0, 7, 3, 0, 7, 5), (None, None, None)),
        ((2, 5, 4, 2, 5, 10), (Some(5), None, op("@flaky"))),
        ((3, 4, 5, 3, 4, 13), (Some(5), None, op("@serial"))),
        ((5, 2, 7, 5, 2, 19), (Some(5), None, None)),
        ((2, 5, 4, 2, 5, 10), (Some(5), Some(secs(5)), op("@flaky"))),
        ((3, 4, 5, 3, 4, 13), (Some(5), Some(secs(5)), op("@serial"))),
        ((5, 2, 7, 5, 2, 19), (Some(5), Some(secs(5)), None)),
    ])
    .then(
        |(
            (p_sc, f_sc, r_sc, p_st, f_st, r_st),
            (retries, retry_after, retry_filter),
        )| async move {
            let now = Instant::now();

            let writer = World::cucumber()
                .retries(retries)
                .retry_after(retry_after)
                .retry_filter(retry_filter)
                .run("tests/features/retry")
                .await;

            let elapsed = now.elapsed();

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

            elapsed
        },
    )
    .enumerate()
    .fold(Duration::ZERO, |prev, (id, cur)| async move {
        if let Some(diff) = prev
            .checked_sub(cur)
            .filter(|diff| *diff > Duration::from_secs(2))
        {
            panic!(
                "Test case at index {} ran longer than the next one \
                 (index {id}) for {}",
                id - 1,
                humantime::Duration::from(diff),
            )
        }
        cur
    })
    .await;

    // for (
    //     (p_sc, f_sc, r_sc, p_st, f_st, r_st, dur),
    //     (retries, retry_after, retry_filter),
    // ) in [
    //     ((0, 7, 3, 0, 7, 5, secs(7)), (None, None, None)),
    //     ((5, 2, 7, 5, 2, 19, secs(15)), (Some(5), None, None)),
    //     (
    //         (5, 2, 7, 5, 2, 19, secs(22)),
    //         (Some(5), Some(secs(5)), None),
    //     ),
    //     ((2, 5, 4, 2, 5, 10, secs(12)), (Some(5), None, op("@flaky"))),
    //     (
    //         (3, 4, 5, 3, 4, 13, secs(15)),
    //         (Some(5), None, op("@serial")),
    //     ),
    //     (
    //         (2, 5, 4, 2, 5, 10, secs(20)),
    //         (Some(5), Some(secs(5)), op("@flaky")),
    //     ),
    //     (
    //         (3, 4, 5, 3, 4, 13, secs(20)),
    //         (Some(5), Some(secs(5)), op("@serial")),
    //     ),
    // ] {
    //     let now = Instant::now();
    //
    //     let writer = World::cucumber()
    //         .retries(retries)
    //         .retry_after(retry_after)
    //         .retry_filter(retry_filter)
    //         .run("tests/features/retry")
    //         .await;
    //
    //     let elapsed = now.elapsed();
    //     let abs_diff =
    //         dur.checked_sub(elapsed).unwrap_or_else(|| elapsed - dur);
    //     assert!(
    //         abs_diff < Duration::from_millis(100),
    //         "Expected time difference is more than 100ms: {}",
    //         humantime::Duration::from(abs_diff),
    //     );
    //
    //     assert_eq!(
    //         writer.scenarios,
    //         Stats {
    //             passed: p_sc,
    //             skipped: 0,
    //             failed: f_sc,
    //             retried: r_sc,
    //         },
    //         "Wrong Stats for Scenarios",
    //     );
    //     assert_eq!(
    //         writer.steps,
    //         Stats {
    //             passed: p_st,
    //             skipped: 0,
    //             failed: f_st,
    //             retried: r_st,
    //         },
    //         "Wrong Stats for Steps",
    //     );
    //
    //     SCENARIO_RUNS.lock().await.clear();
    // }
}

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World;
