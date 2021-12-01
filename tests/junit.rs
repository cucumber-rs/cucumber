use std::{convert::Infallible, fs, io::Read as _};

use async_trait::async_trait;
use cucumber::{given, then, when, writer, WorldInit};
use regex::Regex;
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
            .with_writer(writer::JUnit::new(file.reopen().unwrap()))
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = Regex::new(
        "time(stamp)?=\"[^\"]+\"\
         |: [/\\\\](.*)[/\\\\]([A-z1-9-_]*).feature(:\\d+:\\d+)?\
         |\\s?\n",
    )
    .unwrap();

    assert_eq!(
        non_deterministic.replace_all(&buffer, ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/junit/correct.xml").unwrap(),
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
