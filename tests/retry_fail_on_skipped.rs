use std::io;

use cucumber::{StatsWriter as _, World as _, WriterExt as _, writer};

#[derive(cucumber::World, Clone, Copy, Debug, Default)]
struct World;

#[tokio::test]
async fn fails() {
    // We place `writer::Summarized` in a pipeline before `writer::Normalized`
    // to check whether the later one messes up the ordering.
    let res = World::cucumber()
        .with_writer(
            writer::Basic::raw(
                io::stdout(),
                writer::Coloring::Auto,
                writer::Verbosity::Default,
            )
            .summarized()
            .normalized(),
        )
        .fail_on_skipped()
        .retries(1)
        .with_default_cli()
        .run("tests/features/readme/eating.feature")
        .await;

    assert_eq!(res.passed_steps(), 0);
    assert_eq!(res.skipped_steps(), 0);
    assert_eq!(res.failed_steps(), 1);
    assert_eq!(res.retried_steps(), 0);
    assert_eq!(res.hook_errors(), 0);
}
