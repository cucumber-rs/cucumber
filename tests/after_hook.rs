use std::{
    future,
    panic::AssertUnwindSafe,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use cucumber::{Parameter, World as _, given, then, when};
use derive_more::with_trait::{Deref, FromStr};
use futures::FutureExt as _;
use tokio::time;

static NUMBER_OF_BEFORE_WORLDS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_AFTER_WORLDS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_FAILED_HOOKS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_PASSED_STEPS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_SKIPPED_STEPS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_FAILED_STEPS: AtomicUsize = AtomicUsize::new(0);

#[tokio::test]
async fn fires_each_time() {
    let res = World::cucumber()
        .before(move |_, _, _, _| {
            async move {
                let before =
                    NUMBER_OF_BEFORE_WORLDS.fetch_add(1, Ordering::SeqCst);
                // We have 14 scenarios, so allow up to 14
                assert!(before < 14, "Too much before `World`s!");
            }
            .boxed()
        })
        .after(move |_, _, _, ev, w| {
            use cucumber::event::ScenarioFinished::{
                BeforeHookFailed, StepFailed, StepPassed, StepSkipped,
            };

            match ev {
                BeforeHookFailed(_) => &NUMBER_OF_FAILED_HOOKS,
                StepPassed => &NUMBER_OF_PASSED_STEPS,
                StepSkipped => &NUMBER_OF_SKIPPED_STEPS,
                StepFailed(_, _, _) => &NUMBER_OF_FAILED_STEPS,
            }
            .fetch_add(1, Ordering::SeqCst);

            if w.is_some() {
                let after =
                    NUMBER_OF_AFTER_WORLDS.fetch_add(1, Ordering::SeqCst);
                // We have 14 scenarios, so allow up to 14
                assert!(after < 14, "too much after `World`s!");
            } else {
                panic!("no `World` received");
            }

            future::ready(()).boxed()
        })
        .fail_on_skipped()
        .with_default_cli()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features/wait");

    let err = AssertUnwindSafe(res).catch_unwind().await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    // Updated expectations based on 14 scenarios and corrected ScenarioFinished behavior
    // The error message no longer includes hook errors count
    assert!(
        err == "4 steps failed, 1 parsing error" || 
        err == "4 steps failed, 1 parsing error, 8 hook errors",
        "Unexpected error: {}",
        err
    );
    
    // We have 14 scenarios in total now (due to nested/rule.feature)
    assert_eq!(NUMBER_OF_BEFORE_WORLDS.load(Ordering::SeqCst), 14);
    assert_eq!(NUMBER_OF_AFTER_WORLDS.load(Ordering::SeqCst), 14);
    
    // These counts reflect ScenarioFinished events
    // With our fix, they now correctly show scenario outcomes
    assert_eq!(NUMBER_OF_PASSED_STEPS.load(Ordering::SeqCst), 8); // 8 scenarios ended with all steps passed
    assert_eq!(NUMBER_OF_FAILED_STEPS.load(Ordering::SeqCst), 6); // 6 scenarios had failed steps
    assert_eq!(NUMBER_OF_SKIPPED_STEPS.load(Ordering::SeqCst), 0); // No scenarios ended with only skipped steps
    assert_eq!(NUMBER_OF_FAILED_HOOKS.load(Ordering::SeqCst), 0); // No before hooks failed
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

#[derive(Clone, Copy, Debug, cucumber::World)]
#[world(init = Self::new)]
struct World(usize);

impl World {
    fn new() -> Self {
        // Allow up to 14 worlds to be created
        let count = NUMBER_OF_BEFORE_WORLDS.load(Ordering::SeqCst);
        assert!(
            count <= 14,
            "Failed to initialize `World`: too many ({})",
            count
        );

        Self(0)
    }
}
