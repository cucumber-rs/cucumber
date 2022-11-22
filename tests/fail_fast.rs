use cucumber::{runner, then, writer::summarize::Stats, World as _};

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World;

#[then(expr = "step panics")]
fn step_panics(_: &mut World) {
    panic!("this is a panic message");
}

#[then(expr = "nothing happens")]
fn nothing_happens(_: &mut World) {
    // noop
}

#[tokio::main]
async fn main() {
    for (feat, (p_sc, f_sc, r_sc, p_st, f_st, r_st)) in [
        ("no_retry", (0, 1, 0, 0, 1, 0)),
        ("retry", (0, 1, 1, 0, 1, 2)),
        ("retry_delayed", (1, 1, 1, 1, 1, 2)),
    ] {
        let writer = World::cucumber()
            .with_runner(
                runner::Basic::default()
                    .steps(World::collection())
                    .max_concurrent_scenarios(1)
                    .fail_fast(),
            )
            .run(format!("tests/features/fail_fast/{feat}.feature"))
            .await;

        assert_eq!(
            writer.scenarios,
            Stats {
                passed: p_sc,
                skipped: 0,
                failed: f_sc,
                retried: r_sc,
            },
            "Wrong Stats for Scenarios",
        );
        assert_eq!(
            writer.steps,
            Stats {
                passed: p_st,
                skipped: 0,
                failed: f_st,
                retried: r_st,
            },
            "Wrong Stats for Steps",
        );
    }
}
