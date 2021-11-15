use std::{convert::Infallible, fs, io::Read as _};

use async_trait::async_trait;
use cucumber::{given, then, when, writer, WorldInit, WriterExt as _};
use futures::FutureExt as _;
use regex::{Regex, RegexBuilder};
use tempfile::NamedTempFile;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
fn step(world: &mut World) {
    world.0 += 1;
    if world.0 > 3 {
        panic!("Too much!");
    }
}

#[tokio::main]
async fn main() {
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .before(|_, _, sc, _| {
                async {
                    if sc.tags.iter().any(|t| t == "fail_before") {
                        panic!("Tag!");
                    }
                }
                .boxed_local()
            })
            .after(|_, _, sc, _| {
                async {
                    if sc.tags.iter().any(|t| t == "fail_after") {
                        panic!("Tag!");
                    }
                }
                .boxed_local()
            })
            .with_writer(writer::Json::new(file.reopen().unwrap()))
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = RegexBuilder::new(
        "\"uri\":\\s?\"[^\"]*\"\
         |\"duration\":\\s?\\d+\
         |\"id\":\\s?\"failed[^\"]*\"\
         |\"error_message\":\\s?\"Could[^\"]*\"\
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

#[derive(Clone, Copy, Debug, WorldInit)]
struct World(usize);

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World(0))
    }
}
