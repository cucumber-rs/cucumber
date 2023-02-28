use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use cucumber::{given, then, when, World as _};
use tokio::time;

#[derive(cucumber::World, Debug, Default)]
struct World;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
async fn sleep(_: &mut World, secs: u64) {
    static ID: AtomicUsize = AtomicUsize::new(0);

    let id = ID.fetch_add(1, Ordering::Relaxed);

    tracing::info!("before {secs}s sleep: {id}");
    time::sleep(Duration::from_secs(secs)).await;
    tracing::info!("after {secs}s sleep: {id}");
}

#[tokio::main]
async fn main() {
    World::cucumber()
        .init_tracing()
        .run("tests/features/wait")
        .await;
}
