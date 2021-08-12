//! Default [`Parser`] implementation.

use std::{path::Path, vec};

use futures::stream;
use gherkin::GherkinEnv;
use globwalk::GlobWalkerBuilder;

use super::Parser;

/// Default [`Parser`].
///
/// As there is no async runtime-agnostic way to interact with IO, this
/// [`Parser`] is blocking.
#[derive(Clone, Copy, Debug)]
pub struct Basic;

impl<I: AsRef<Path>> Parser<I> for Basic {
    type Output = stream::Iter<vec::IntoIter<gherkin::Feature>>;

    fn parse(self, path: I) -> Self::Output {
        let path = path
            .as_ref()
            .canonicalize()
            .expect("failed to canonicalize path");

        let features = if path.is_file() {
            let env = GherkinEnv::default();
            gherkin::Feature::parse_path(path, env).map(|f| vec![f])
        } else {
            let walker = GlobWalkerBuilder::new(path, "*.feature")
                .case_insensitive(true)
                .build()
                .unwrap();
            walker
                .filter_map(Result::ok)
                .map(|entry| {
                    let env = GherkinEnv::default();
                    gherkin::Feature::parse_path(entry.path(), env)
                })
                .collect::<Result<_, _>>()
        }
        .expect("failed to parse gherkin::Feature");

        stream::iter(features)
    }
}
