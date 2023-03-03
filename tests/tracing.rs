use std::{fs, io, panic::AssertUnwindSafe, time::Duration};

use cucumber::{given, writer, writer::Coloring, World as _, WriterExt as _};
use derive_more::Display;
use futures::FutureExt as _;
use regex::Regex;
use tokio::{spawn, time};
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::format::{DefaultFields, Format},
    layer::SubscriberExt as _,
    Layer,
};

#[tokio::main]
async fn main() {
    spawn(async {
        let mut id = 0;
        loop {
            time::sleep(Duration::from_millis(600)).await;
            tracing::info!("not in span: {id}");
            id += 1;
        }
    });

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

    AssertUnwindSafe(res).catch_unwind().await.unwrap();

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
async fn step(_: &mut World, n: usize) {
    time::sleep(Duration::from_secs(1)).await;
    tracing::info!("in span: {n:?}");
}

#[derive(Clone, cucumber::World, Debug, Default, Display)]
struct World;
