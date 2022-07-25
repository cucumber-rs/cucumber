use std::{borrow::Cow, cmp::Ordering, fmt::Debug};

use async_trait::async_trait;
use cucumber::{cli, event, given, parser, step, then, when, Event, Writer};
use itertools::Itertools as _;
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
struct DebugWriter(String);

#[async_trait(?Send)]
impl<World: 'static + Debug> Writer<World> for DebugWriter {
    type Cli = cli::Empty;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<World>>>,
        _: &Self::Cli,
    ) {
        use event::{Cucumber, Feature, Rule, Scenario, Step, StepError};

        // This function is used to provide a deterministic ordering of
        // `possible_matches`.
        let sort_matches = |mut e: step::AmbiguousMatchError| {
            e.possible_matches = e
                .possible_matches
                .into_iter()
                .sorted_by(|(re_l, loc_l), (re_r, loc_r)| {
                    let re_ord = Ord::cmp(re_l, re_r);
                    if re_ord != Ordering::Equal {
                        return re_ord;
                    }
                    loc_l
                        .as_ref()
                        .and_then(|l| loc_r.as_ref().map(|r| Ord::cmp(l, r)))
                        .unwrap_or(Ordering::Equal)
                })
                .collect();
            e
        };

        let ev: Cow<_> = match ev.map(Event::into_inner) {
            Err(_) => "ParsingError".into(),
            Ok(Cucumber::Feature(
                feat,
                Feature::Rule(
                    rule,
                    Rule::Scenario(
                        sc,
                        Scenario::Step(
                            st,
                            Step::Failed(
                                cap,
                                loc,
                                w,
                                StepError::AmbiguousMatch(e),
                            ),
                        ),
                    ),
                ),
            )) => {
                let ev = Cucumber::scenario(
                    feat,
                    Some(rule),
                    sc,
                    Scenario::Step(
                        st,
                        Step::Failed(
                            cap,
                            loc,
                            w,
                            StepError::AmbiguousMatch(sort_matches(e)),
                        ),
                    ),
                );

                format!("{ev:?}").into()
            }
            Ok(Cucumber::Feature(
                feat,
                Feature::Scenario(
                    sc,
                    Scenario::Step(
                        st,
                        Step::Failed(cap, loc, w, StepError::AmbiguousMatch(e)),
                    ),
                ),
            )) => {
                let ev = Cucumber::scenario(
                    feat,
                    None,
                    sc,
                    Scenario::Step(
                        st,
                        Step::Failed(
                            cap,
                            loc,
                            w,
                            StepError::AmbiguousMatch(sort_matches(e)),
                        ),
                    ),
                );

                format!("{ev:?}").into()
            }
            Ok(ev) => format!("{ev:?}").into(),
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
         |, col: (\\d+)\
         | path: (None|(Some\\()?\"[^\"]*\")\\)?,?)",
    )
    .unwrap()
});

#[cfg(test)]
mod spec {
    use std::fs;

    use cucumber::{World as _, WriterExt as _};
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

        assert_eq!(
            files.len(),
            fs::read_dir("tests/features/output").unwrap().count() / 2,
            "Not all `.feature` files were collected",
        );

        for file in files {
            let out = fs::read_to_string(format!(
                "tests/features/output/{file}.out",
            ))
            .unwrap_or_default()
            .lines()
            .collect::<String>();
            let normalized = World::cucumber()
                .with_writer(DebugWriter::default().normalized())
                .run(format!("tests/features/output/{file}"))
                .await;

            assert_eq!(normalized.0, out, "file: {file}");
        }
    }
}
