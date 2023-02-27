use std::{fs, io, panic::AssertUnwindSafe, thread};

use cucumber::{
    given, then, when, writer, writer::Coloring, World as _, WriterExt as _,
};
use derive_more::Display;
use futures::FutureExt as _;
use regex::Regex;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::format::{DefaultFields, Format},
    layer::SubscriberExt as _,
    Layer,
};

#[tokio::main]
async fn main() {
    let mut out = Vec::<u8>::new();

    let res = World::cucumber()
        .with_writer(
            writer::Basic::raw(&mut out, Coloring::Never, 0)
                .discard_stats_writes()
                .tee::<World, _>(
                    writer::Basic::raw(io::stdout(), Coloring::Never, 0)
                        .summarized(),
                )
                .normalized(),
        )
        .fail_on_skipped()
        .with_default_cli()
        .configure_and_init_tracing(
            DefaultFields::new(),
            Format::default().with_ansi(false).without_time(),
            |layer| {
                tracing_subscriber::registry()
                    .with(LevelFilter::INFO.and_then(layer))
            },
        )
        .run_and_exit("tests/features/tracing");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();
    assert_eq!(err, "1 step failed");

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = Regex::new(
        " ([^\"\\n\\s]*)[/\\\\]([A-z1-9-_]*)\\.(feature|rs)(:\\d+:\\d+)?\
             |\\s?\n",
    )
    .unwrap();

    assert_eq!(
        non_deterministic
            .replace_all(String::from_utf8_lossy(&out).as_ref(), ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/features/tracing/correct.stdout")
                .unwrap(),
            "",
        ),
    );
}

#[given(regex = "step (\\d+)")]
#[when(regex = "step (\\d+)")]
#[then(regex = "step (\\d+)")]
fn step(world: &mut World, n: usize) {
    tracing::info!("before increment: {world}: {n:?}");

    world.counter += 1;

    let span = tracing::Span::current();
    thread::scope(|s| {
        s.spawn(|| {
            let _guard = span.enter();
            tracing::info!("after increment in `Span`: {world}: {n:?}");
        })
        .join()
        .unwrap();
        s.spawn(|| {
            tracing::info!("after increment without `Span`: {world}: {n:?}");
        })
        .join()
        .unwrap();
    });
}

#[derive(Clone, cucumber::World, Debug, Default, Display)]
struct World {
    counter: usize,
}
