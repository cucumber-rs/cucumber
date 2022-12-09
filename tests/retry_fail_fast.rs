use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

use cucumber::{given, StatsWriter as _, World};

#[derive(Clone, Copy, Debug, Default, World)]
struct W;

#[given(regex = "attempt (\\d+)")]
async fn assert(_: &mut W) {
    static TIMES_CALLED: AtomicUsize = AtomicUsize::new(0);

    match TIMES_CALLED.fetch_add(1, SeqCst) {
        n @ 1..=5 if n % 2 != 0 => panic!("flake"),
        0..=5 => {}
        _ => panic!("too much!"),
    }
}

#[tokio::main]
async fn main() {
    let writer = W::cucumber()
        .max_concurrent_scenarios(1)
        .retries(3)
        .fail_fast()
        .run("tests/features/retry_fail_fast")
        .await;
    assert_eq!(writer.passed_steps(), 3);
    assert_eq!(writer.skipped_steps(), 0);
    assert_eq!(writer.failed_steps(), 1);
    assert_eq!(writer.retried_steps(), 5);
    assert_eq!(writer.parsing_errors(), 0);
    assert_eq!(writer.hook_errors(), 0);
}
