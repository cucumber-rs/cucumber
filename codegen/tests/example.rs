use std::{convert::Infallible, time::Duration};

use async_trait::async_trait;
use cucumber_rust::{gherkin::Step, given, World, WorldInit, WorldRun as _};
use tokio::time;

#[derive(Debug, WorldInit)]
pub struct MyWorld {
    foo: i32,
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self { foo: 0 })
    }
}

#[given("non-regex")]
fn test_non_regex_sync(w: &mut MyWorld) {
    w.foo += 1;
}

#[given("non-regex")]
async fn test_non_regex_async(w: &mut MyWorld, #[given(step)] ctx: &Step) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(ctx.value, "non-regex");

    w.foo += 1;
}

#[given(regex = r"(\S+) is (\d+)")]
async fn test_regex_async(
    w: &mut MyWorld,
    step: String,
    #[given(step)] ctx: &Step,
    num: usize,
) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(step, "foo");
    assert_eq!(num, 0);
    assert_eq!(ctx.value, "foo is 0");

    w.foo += 1;
}

#[given(regex = r"(\S+) is sync (\d+)")]
fn test_regex_sync_slice(w: &mut MyWorld, step: &Step, matches: &[String]) {
    assert_eq!(matches[0], "foo");
    assert_eq!(matches[1].parse::<usize>().unwrap(), 0);
    assert_eq!(step.value, "foo is sync 0");

    w.foo += 1;
}

#[tokio::main]
async fn main() {
    MyWorld::run("./tests/features").await;
}
