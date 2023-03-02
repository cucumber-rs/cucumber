use std::{fs, io::Read as _};

use cucumber::{given, then, when, writer, World as _};
use futures::FutureExt as _;
use regex::RegexBuilder;
use tempfile::NamedTempFile;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::format::{DefaultFields, Format},
    layer::SubscriberExt as _,
    Layer as _,
};

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
fn step(world: &mut World) {
    tracing::info!("step");
    world.0 += 1;
    tracing::info!("world: {world:?}");
    assert!(world.0 < 4, "Too much!");
}

#[tokio::test]
async fn test() {
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .before(|_, _, sc, _| {
                async {
                    tracing::info!("before");
                    assert!(
                        !(sc.name == "wait"
                            && sc.tags.iter().any(|t| t == "fail_before")),
                        "Tag!",
                    );
                }
                .boxed_local()
            })
            .after(|_, _, sc, _, _| {
                async {
                    tracing::info!("after");
                    assert!(!sc.tags.iter().any(|t| t == "fail_after"), "Tag!");
                }
                .boxed_local()
            })
            .with_writer(writer::Json::new(file.reopen().unwrap()))
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
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = RegexBuilder::new(
        "\"duration\":\\s?\\d+\
         |([^\"\\n\\s]*)[/\\\\]([A-z1-9-_]*)\\.(feature|rs)(:\\d+:\\d+)?\
         |\n\
         |\\s",
    )
    .multi_line(true)
    .build()
    .unwrap();

    assert_eq!(
        non_deterministic.replace_all(&buffer, ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/json/correct.json").unwrap(),
            "",
        ),
    );
}

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World(usize);
