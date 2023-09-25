use std::{fs, io::Read as _};

use cucumber::{cli, given, then, when, writer, World as _, WriterExt as _};
use regex::Regex;
use tempfile::NamedTempFile;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
fn step(world: &mut World) {
    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}

#[tokio::test]
async fn output() {
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .with_writer(
                writer::Libtest::new(file.reopen().unwrap()).normalized(),
            )
            .fail_on_skipped()
            .with_default_cli()
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = Regex::new(
        "\":\\d+\\.\\d+\
         |([^\"\\n\\s]*)[/\\\\]([A-z1-9-_]*)\\.(feature|rs)(:\\d+:\\d+)?\
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

#[tokio::test]
async fn output_report_time() {
    let mut cli = cli::Opts::<_, _, writer::libtest::Cli>::default();
    cli.writer.report_time = Some(writer::libtest::ReportTime::Plain);
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .with_writer(
                writer::Libtest::new(file.reopen().unwrap()).normalized(),
            )
            .fail_on_skipped()
            .with_cli(cli)
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = Regex::new(
        "\":\\d+\\.\\d+\
         |([^\"\\n\\s]*)[/\\\\]([A-z1-9-_]*)\\.(feature|rs)(:\\d+:\\d+)?\
         |\\s?\n",
    )
    .unwrap();

    assert_eq!(
        non_deterministic.replace_all(&buffer, ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/libtest/correct-report-time.stdout")
                .unwrap(),
            "",
        ),
    );
}

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World(usize);
