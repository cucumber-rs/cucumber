use std::{convert::Infallible, panic::AssertUnwindSafe, time::Duration};

use async_trait::async_trait;
use cucumber::{cli, given, then, when, WorldInit};
use futures::FutureExt as _;
use structopt::StructOpt as _;
use tokio::time;

#[tokio::main]
async fn main() {
    let (_custom, cli) =
        cli::Compose::<cli::Empty, cli::Opts<_, _, _>>::from_args()
            .into_inner();

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
        .run_and_exit_with_cli(cli, "tests/features/wait");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "2 steps failed, 1 parsing error");
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
