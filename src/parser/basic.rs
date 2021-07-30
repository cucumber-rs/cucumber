//! Default [`Parser`] implementation.

use std::{mem, path::Path, vec};

use futures::stream;

use crate::Parser;

/// Default [`Parser`].
///
/// As there is no async runtime-agnostic way to interact with io, this
/// [`Parser`] is blocking.
#[derive(Clone, Copy, Debug)]
pub struct Basic;

impl<I, F> Parser<I, F> for Basic
where
    I: AsRef<Path>,
    F: Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
    ) -> bool,
{
    type Output = stream::Iter<vec::IntoIter<gherkin::Feature>>;

    fn parse(self, path: I, filter: Option<F>) -> Self::Output {
        let path = path
            .as_ref()
            .canonicalize()
            .expect("failed to canonicalize path");

        let mut features = if path.is_file() {
            let env = gherkin::GherkinEnv::default();
            gherkin::Feature::parse_path(path, env).map(|f| vec![f])
        } else {
            let walker = globwalk::GlobWalkerBuilder::new(path, "*.feature")
                .case_insensitive(true)
                .build()
                .unwrap();
            walker
                .filter_map(Result::ok)
                .map(|entry| {
                    let env = gherkin::GherkinEnv::default();
                    gherkin::Feature::parse_path(entry.path(), env)
                })
                .collect::<Result<_, _>>()
        }
        .expect("failed to parse gherkin::Feature");

        for f in &mut features {
            let scenarios = mem::take(&mut f.scenarios);
            f.scenarios = scenarios
                .into_iter()
                .filter(|s| {
                    filter.as_ref().map_or(true, |filter| filter(f, None, s))
                })
                .collect();

            let mut rules = mem::take(&mut f.rules);
            for r in &mut rules {
                let scenarios = mem::take(&mut r.scenarios);
                r.scenarios = scenarios
                    .into_iter()
                    .filter(|s| {
                        filter
                            .as_ref()
                            .map_or(true, |filter| filter(f, Some(r), s))
                    })
                    .collect();
            }
            f.rules = rules;
        }

        stream::iter(features)
    }
}
