use std::panic::AssertUnwindSafe;

use cucumber::World as _;
use futures::FutureExt as _;

#[derive(cucumber::World, Clone, Copy, Debug, Default)]
struct World;

#[tokio::main]
async fn main() {
    let res = World::cucumber()
        .fail_on_skipped()
        .retries(1)
        .run_and_exit("tests/features/readme/eating.feature");
    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "1 step failed");
}
