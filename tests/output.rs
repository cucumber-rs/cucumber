use std::{borrow::Cow, convert::Infallible, fmt::Debug};

use async_trait::async_trait;
use cucumber::{event, given, parser, then, when, WorldInit, Writer};
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Default, WorldInit)]
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

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World::default())
    }
}

#[derive(Default)]
struct DebugWriter(String);

#[async_trait(?Send)]
impl<World: 'static + Debug> Writer<World> for DebugWriter {
    async fn handle_event(
        &mut self,
        ev: parser::Result<event::Cucumber<World>>,
    ) {
        let ev: Cow<_> = match ev {
            Err(_) => "ParsingError".into(),
            Ok(ev) => format!("{:?}", ev).into(),
        };

        let without_span = SPAN_OR_PATH_RE.replace_all(ev.as_ref(), "");

        self.0.push_str(without_span.as_ref());
    }
}

/// [`Regex`] to unify spans and file paths on Windows, Linux and macOS for
/// tests.
static SPAN_OR_PATH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "( span: Span \\{ start: (\\d+), end: (\\d+) },\
         | path: (None|(Some\\()?\"[^\"]*\")\\)?,?)",
    )
    .unwrap()
});

#[cfg(test)]
mod spec {
    use std::fs;

    use cucumber::{WorldInit as _, WriterExt as _};
    use globwalk::GlobWalkerBuilder;

    use super::{DebugWriter, World};

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

        for file in files {
            let out = fs::read_to_string(format!(
                "tests/features/output/{}.out",
                file,
            ))
            .unwrap_or_default()
            .lines()
            .collect::<String>();
            let normalized = World::cucumber()
                .with_writer(DebugWriter::default().normalized())
                .run(format!("tests/features/output/{}", file))
                .await;

            assert_eq!(normalized.0, out, "file: {}", file);
        }
    }
}
