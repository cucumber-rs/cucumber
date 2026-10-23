use std::{
    fs::File,
    io::{Read as _, Seek as _, SeekFrom},
};

use cucumber::{
    World, given,
    writer::{self, Basic, Summarize, basic::Coloring},
};

#[given("ok")]
fn ok(_: &mut W) {}

#[given("error")]
fn error(_: &mut W) -> Result<(), &'static str> {
    Err("error")
}

#[derive(Clone, Copy, Debug, Default, World)]
struct W;

#[tokio::test]
async fn summary_lists_failed_scenarios() {
    let mut output = tempfile::tempfile().unwrap();
    let writer = Summarize::new(Basic::new(
        output.try_clone().unwrap(),
        Coloring::Never,
        writer::Verbosity::Default,
    ));

    _ = W::cucumber()
        .with_writer(writer)
        .with_default_cli()
        .run("tests/features/failed_scenarios")
        .await;

    let mut printed = String::new();
    output.seek(SeekFrom::Start(0)).unwrap();
    _ = File::read_to_string(&mut output, &mut printed).unwrap();
    let summary = printed.split("[Summary]").nth(1).unwrap();

    let (header, failed) = summary.split_once("Failed scenarios:\n").unwrap();
    assert!(header.contains("3 scenarios (1 passed, 2 failed)"), "{summary}");

    let mut lines = failed.lines();
    assert!(
        lines.next().unwrap().ends_with(".feature:6: Failures › Error"),
        "{summary}",
    );
    assert!(
        lines
            .next()
            .unwrap()
            .ends_with(".feature:11: Failures › Rule › Error in rule"),
        "{summary}",
    );
    assert_eq!(lines.next(), None, "{summary}");
}
