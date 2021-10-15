use std::{convert::Infallible, panic::AssertUnwindSafe, time::Duration};

use async_trait::async_trait;
use cucumber::{given, then, when, WorldInit};
use futures::FutureExt as _;
use tokio::time;

#[tokio::main]
async fn main() {
    let res = World::cucumber()
        .before(|_, _, _, w| {
            async move {
                w.0 = 0;
                time::sleep(Duration::from_millis(10)).await;
            }
            .boxed_local()
        })
        .after(|_, _, _, _| {
            time::sleep(Duration::from_millis(10)).boxed_local()
        })
        .run_and_exit("tests/features/wait");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "2 steps failed, 1 parsing error, 0 hook errors");
}

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
async fn step(world: &mut World, secs: u64) {
    time::sleep(Duration::from_secs(secs)).await;

    world.0 += 1;
    if world.0 > 3 {
        panic!("Too much!");
    }
}

#[derive(Clone, Copy, Debug, WorldInit)]
struct World(usize);

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World(0))
    }
}
