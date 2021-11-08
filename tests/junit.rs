use std::{convert::Infallible, fs, io::Read as _, time::Duration};

use async_trait::async_trait;
use cucumber::{given, then, when, writer, WorldInit, WriterExt as _};
use regex::Regex;
use tempfile::NamedTempFile;
use tokio::time;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
async fn step(world: &mut World, secs: u64) {
    time::sleep(Duration::from_secs(secs)).await;

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
            .with_writer(
                writer::JUnit::new(file.reopen().unwrap()).normalized(),
            )
            .run("tests/features/wait")
            .await,
    );

    let non_deterministic = Regex::new(
        r#"time(stamp)?="[^"]+"|: [/A-z]+.feature(:\d+:\d+)?|:\s?\n"#,
    )
    .unwrap();

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    assert_eq!(
        non_deterministic.replace_all(&buffer, ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/xml/correct.xml").unwrap(),
            "",
        )
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
