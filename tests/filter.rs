use std::{fmt, io};

use cucumber::{
    StatsWriter, World as _, WriterExt as _, given, then, when, writer,
};

#[given(regex = r"(\d+) < 10")]
#[when(regex = r"(\d+) < 10")]
#[then(regex = r"(\d+) < 10")]
fn step(_: &mut World, num: usize) {
    assert!(num < 10, "not filtered");
}

#[tokio::test]
async fn by_examples() {
    let mut output = Output::default();
    let writer = World::cucumber()
        .with_writer(
            writer::Basic::new(&mut output, writer::Coloring::Auto, 0)
                .summarized(),
        )
        .with_default_cli()
        .filter_run("tests/features/filter", |_, _, sc| {
            // Omit `Examples` rows containing numbers less than 10.
            (sc.name == "by examples")
                && (sc.examples.first().is_some_and(|example| {
                    example.table.as_ref().is_some_and(|table| {
                        table.rows.get(1).is_some_and(|cols| {
                            cols.iter().all(|v| {
                                v.parse::<usize>().is_ok_and(|num| num < 10)
                            })
                        })
                    })
                }))
        })
        .await;

    if writer.execution_has_failed() {
        panic!("some steps failed:\n{output}");
    }
}

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World;

/// Deterministic output of [`writer::Basic`].
#[derive(Clone, Debug, Default)]
struct Output(Vec<u8>);

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = String::from_utf8(self.0.clone())
            .unwrap_or_else(|e| panic!("`Output` is not a string: {e}"));
        write!(f, "{o}")
    }
}
