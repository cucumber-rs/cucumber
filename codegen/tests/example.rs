use std::{convert::Infallible, time::Duration};

use async_trait::async_trait;
use cucumber_rust::{given, StepContext, World, WorldInit};
use tokio::time;

#[derive(WorldInit)]
pub struct MyWorld {
    pub foo: i32,
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self { foo: 0 })
    }
}

#[given(regex = r"(\S+) is (\d+)")]
async fn test_regex_async(
    w: &mut MyWorld,
    step: String,
    #[given(context)] ctx: &StepContext,
    num: usize,
) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(step, "foo");
    assert_eq!(num, 0);
    assert_eq!(ctx.step.value, "foo is 0");

    w.foo += 1;
}

#[given(regex = r"(\S+) is sync (\d+)")]
async fn test_regex_sync(
    w: &mut MyWorld,
    s: String,
    #[given(context)] ctx: &StepContext,
    num: usize,
) {
    assert_eq!(s, "foo");
    assert_eq!(num, 0);
    assert_eq!(ctx.step.value, "foo is sync 0");

    w.foo += 1;
}

#[tokio::main]
async fn main() {
    let runner = MyWorld::init(&["./tests/features"]);
    runner.run().await;
}
