use std::{
    panic::AssertUnwindSafe,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use cucumber::{given, then, when, Parameter, World as _};
use derive_more::{Deref, FromStr};
use futures::FutureExt as _;
use tokio::time;

static NUMBER_OF_BEFORE_WORLDS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_AFTER_WORLDS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_FAILED_HOOKS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_PASSED_STEPS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_SKIPPED_STEPS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_FAILED_STEPS: AtomicUsize = AtomicUsize::new(0);

#[tokio::main]
async fn main() {
    let res = World::cucumber()
        .before(move |_, _, _, _| {
            async move {
                let before =
                    NUMBER_OF_BEFORE_WORLDS.fetch_add(1, Ordering::SeqCst);
                assert_ne!(before, 8, "Too much before `World`s!");
            }
            .boxed()
        })
        .after(move |_, _, _, ev, w| {
            use cucumber::event::ScenarioFinished::{
                HookFailed, StepFailed, StepPassed, StepSkipped,
            };

            match ev {
                HookFailed(_) => &NUMBER_OF_FAILED_HOOKS,
                StepPassed => &NUMBER_OF_PASSED_STEPS,
                StepSkipped => &NUMBER_OF_SKIPPED_STEPS,
                StepFailed(_, _, _) => &NUMBER_OF_FAILED_STEPS,
            }
            .fetch_add(1, Ordering::SeqCst);

            if w.is_some() {
                let after =
                    NUMBER_OF_AFTER_WORLDS.fetch_add(1, Ordering::SeqCst);
                assert_ne!(after, 8, "Too much after `World`s!");
            } else {
                panic!("No World received");
            }

            async {}.boxed()
        })
        .run_and_exit("tests/features/wait");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "2 steps failed, 1 parsing error, 4 hook errors");
    assert_eq!(NUMBER_OF_BEFORE_WORLDS.load(Ordering::SeqCst), 11);
    assert_eq!(NUMBER_OF_AFTER_WORLDS.load(Ordering::SeqCst), 11);
    assert_eq!(NUMBER_OF_FAILED_HOOKS.load(Ordering::SeqCst), 2);
    assert_eq!(NUMBER_OF_PASSED_STEPS.load(Ordering::SeqCst), 6);
    assert_eq!(NUMBER_OF_SKIPPED_STEPS.load(Ordering::SeqCst), 2);
    assert_eq!(NUMBER_OF_FAILED_STEPS.load(Ordering::SeqCst), 2);
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

#[derive(Clone, Copy, cucumber::World, Debug)]
#[world(init = Self::new)]
struct World(usize);

impl World {
    fn new() -> Self {
        assert_ne!(
            NUMBER_OF_BEFORE_WORLDS.load(Ordering::SeqCst),
            11,
            "Failed to initialize `World`",
        );

        Self(0)
    }
}
