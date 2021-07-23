use std::{convert::Infallible, panic::AssertUnwindSafe, time::Duration};

use async_trait::async_trait;
use cucumber_rust::{self as cucumber, step, Cucumber};
use futures::{future::LocalBoxFuture, FutureExt as _};
use regex::Regex;
use tokio::time;

#[tokio::main]
async fn main() {
    let re = Regex::new(r"(\d+) secs?").unwrap();

    let res = Cucumber::new()
        .given(re.clone(), step)
        .when(re.clone(), step)
        .then(re, step)
        .run_and_exit("tests");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "2 steps failed");
}

// Unfortunately, we'll still have to generate additional wrapper-function with
// proc-macros due to mysterious "one type is more general than the other" error
//
// MRE: https://bit.ly/3Bv4buB
fn step(world: &mut World, mut ctx: step::Context) -> LocalBoxFuture<()> {
    let f = async move {
        let secs = ctx.matches.pop().unwrap().parse::<u64>().unwrap();
        time::sleep(Duration::from_secs(secs)).await;

        world.0 += 1;
        if world.0 > 3 {
            panic!("Too much!");
        }
    };

    f.boxed_local()
}

#[derive(Clone, Copy, Debug)]
struct World(usize);

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World(0))
    }
}
