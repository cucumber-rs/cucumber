use std::{borrow::Cow, fmt::Debug, mem};

use async_trait::async_trait;
use cucumber::{cli, event, given, parser, then, when, Event, Writer};
use lazy_regex::regex;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(cucumber::World, Debug, Default)]
struct World(usize);

#[given(regex = r"foo is (\d+)")]
#[when(regex = r"foo is (\d+)")]
#[then(regex = r"foo is (\d+)")]
fn step(w: &mut World, num: usize) {
    assert_eq!(w.0, num);
    w.0 += 1;
}

#[given(regex = r"foo is (\d+) ambiguous")]
fn ambiguous(_w: &mut World) {}

#[derive(Default)]
struct DebugWriter {
    output: String,
    first_line_printed: bool,
}

#[async_trait(?Send)]
impl<World: 'static + Debug> Writer<World> for DebugWriter {
    type Cli = cli::Empty;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<World>>>,
        _: &Self::Cli,
    ) {
        let ev: Cow<_> = match ev.map(Event::into_inner) {
            Err(_) => "ParsingError".into(),
            Ok(ev) => format!("{ev:?}").into(),
        };

        let without_span = SPAN_OR_PATH_RE.replace_all(ev.as_ref(), "");

        if mem::replace(&mut self.first_line_printed, true) {
            self.output.push('\n');
        }
        self.output.push_str(without_span.as_ref());
    }
}

/// [`Regex`] to unify spans and file paths on Windows, Linux and macOS for
/// tests.
static SPAN_OR_PATH_RE: &Lazy<Regex> = regex!(
    "( span: Span \\{ start: (\\d+), end: (\\d+) },\
     |, col: (\\d+)\
     | path: (None|(Some\\()?\"[^\"]*\")\\)?,?)"
);

#[cfg(test)]
mod spec {
    use std::fs;

    use cucumber::{
        writer::{self, Coloring},
        World as _, WriterExt as _,
    };
    use globwalk::GlobWalkerBuilder;

    use super::{DebugWriter, World};

    fn load_file(path: impl AsRef<str>) -> Vec<u8> {
        let path = path.as_ref();
        fs::read(path).unwrap_or_else(|| panic!("Failed to load `{path}` file"))
    }

    #[tokio::test]
    async fn test() {
        let walker =
            GlobWalkerBuilder::new("tests/features/output", "*.feature")
                .case_insensitive(true)
                .build()
                .unwrap();
        let files = walker
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_str().unwrap().to_owned())
            .collect::<Vec<String>>();

        assert_eq!(
            files.len(),
            fs::read_dir("tests/features/output").unwrap().count() / 4,
            "Not all `.feature` files were collected",
        );

        for file in files {
            let expected =
                load_file(format!("tests/features/output/{file}.debug.out"));
            let debug = World::cucumber()
                .with_writer(DebugWriter::default().normalized())
                .with_default_cli()
                .run(format!("tests/features/output/{file}"))
                .await;
            assert_eq!(
                debug.output.clone().into_bytes(),
                expected,
                "\n[debug] file: {file}"
            );

            let expected =
                load_file(format!("tests/features/output/{file}.basic.out"));
            let mut actual = Vec::new();
            let _ = World::cucumber()
                .with_writer(
                    writer::Basic::raw(&mut actual, Coloring::Never, 0)
                        .discard_stats_writes()
                        .normalized(),
                )
                .with_default_cli()
                .run(format!("tests/features/output/{file}"))
                .await;
            assert_eq!(actual, expected, "\n[basic] file: {file}");

            let expected =
                load_file(format!("tests/features/output/{file}.colored.out"));
            let mut actual = Vec::new();
            let _ = World::cucumber()
                .with_writer(
                    writer::Basic::raw(&mut actual, Coloring::Always, 0)
                        .discard_stats_writes()
                        .normalized(),
                )
                .with_default_cli()
                .run(format!("tests/features/output/{file}"))
                .await;
            assert_eq!(actual, expected, "\n[colored] file: {file}");
        }
    }
}
