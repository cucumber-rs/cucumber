use std::{fs, io::Read as _};

use cucumber::{given, then, when, writer, World as _};
use regex::RegexBuilder;
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
            .with_writer(writer::JUnit::new(file.reopen().unwrap(), 1))
            .fail_on_skipped()
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = RegexBuilder::new(
        "time(stamp)?=\"[^\"]+\"\
         |: [^\\.\\s]*\\.(feature|rs)(:\\d+:\\d+)?\
         |^\\s+\
         |\\s?\\n",
    )
    .multi_line(true)
    .build()
    .unwrap();

    assert_eq!(
        non_deterministic.replace_all(&buffer, ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/junit/correct.xml").unwrap(),
            "",
        ),
    );
}

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World(usize);
