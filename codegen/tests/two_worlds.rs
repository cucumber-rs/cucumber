use std::time::Duration;

use cucumber::{gherkin::Step, given, when, World};
use tokio::time;

#[derive(Debug, Default, World)]
pub struct FirstWorld {
    foo: i32,
}

#[derive(Debug, Default, World)]
pub struct SecondWorld {
    foo: i32,
}

#[given(regex = r"(\S+) is (\d+)")]
#[when(regex = r"(\S+) is (\d+)")]
async fn test_regex_async(
    w: &mut FirstWorld,
    step: String,
    #[step] ctx: &Step,
    num: usize,
) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(step, "foo");
    assert_eq!(num, 0);
    assert_eq!(ctx.value, "foo is 0");

    w.foo += 1;
}

#[given(regex = r"(\S+) is sync (\d+)")]
fn test_regex_sync_slice(w: &mut SecondWorld, step: &Step, matches: &[String]) {
    assert_eq!(matches[0], "foo");
    assert_eq!(matches[1].parse::<usize>().unwrap(), 0);
    assert_eq!(step.value, "foo is sync 0");

    w.foo += 1;
}

#[tokio::main]
async fn main() {
    let writer = FirstWorld::cucumber()
        .max_concurrent_scenarios(None)
        .run("./tests/features")
        .await;

    assert_eq!(writer.steps.passed, 7);
    assert_eq!(writer.steps.skipped, 5);
    assert_eq!(writer.steps.failed, 0);

    let writer = SecondWorld::cucumber()
        .max_concurrent_scenarios(None)
        .run("./tests/features")
        .await;

    assert_eq!(writer.steps.passed, 1);
    assert_eq!(writer.steps.skipped, 8);
    assert_eq!(writer.steps.failed, 0);
}
