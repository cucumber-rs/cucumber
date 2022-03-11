use std::{
    convert::Infallible,
    panic::AssertUnwindSafe,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use cucumber::{given, then, when, Parameter, WorldInit};
use derive_more::{Deref, FromStr};
use futures::FutureExt as _;
use tokio::time;

#[tokio::main]
async fn main() {
    static NUMBER_OF_WORLDS: AtomicUsize = AtomicUsize::new(0);

    let res = World::cucumber()
        .after(move |_, _, _, w| {
            async move {
                if w.is_some() {
                    NUMBER_OF_WORLDS.fetch_add(1, Ordering::SeqCst);
                }
            }
            .boxed()
        })
        .run_and_exit("tests/features/wait");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "2 steps failed, 1 parsing error");
    assert_eq!(NUMBER_OF_WORLDS.load(Ordering::SeqCst), 12);
}

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(expr = "{u64} sec(s)")]
async fn step(world: &mut World, secs: CustomU64) {
    time::sleep(Duration::from_secs(*secs)).await;

    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}

#[derive(Deref, FromStr, Parameter)]
#[param(regex = "\\d+", name = "u64")]
struct CustomU64(u64);

#[derive(Clone, Copy, Debug, WorldInit)]
struct World(usize);

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World(0))
    }
}
