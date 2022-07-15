use std::{fs, io::Read as _};

use cucumber::{given, then, when, writer, World as _, WriterExt as _};
use regex::Regex;
use tempfile::NamedTempFile;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
fn step(world: &mut World) {
    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}

#[tokio::main]
async fn main() {
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .with_writer(
                writer::Libtest::new(file.reopen().unwrap()).normalized(),
            )
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = Regex::new(
        "\"exec_time\":\\d*\\.?\\d*\
             | [/\\\\]([^\"]*)[/\\\\]([A-z1-9-_]*).feature\
             |\\s?\n",
    )
    .unwrap();

    assert_eq!(
        non_deterministic.replace_all(&buffer, ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/libtest/correct.stdout").unwrap(),
            "",
        ),
    );
}

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World(usize);
