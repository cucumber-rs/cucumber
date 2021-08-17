// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Default [`Parser`] implementation.

use std::{path::Path, vec};

use futures::stream;
use gherkin::GherkinEnv;
use globwalk::GlobWalkerBuilder;

use super::{Parser, Result as ParseResult};

/// Default [`Parser`].
///
/// As there is no async runtime-agnostic way to interact with IO, this
/// [`Parser`] is blocking.
#[derive(Clone, Copy, Debug)]
pub struct Basic;

impl<I: AsRef<Path>> Parser<I> for Basic {
    type Output = stream::Iter<vec::IntoIter<ParseResult<gherkin::Feature>>>;

    fn parse(self, path: I) -> Self::Output {
        let features = || {
            let path = match path.as_ref().canonicalize() {
                Ok(p) => p,
                Err(err) => {
                    return vec![Err(gherkin::ParseFileError::Reading {
                        path: path.as_ref().to_path_buf(),
                        source: err,
                    })];
                }
            };

            if path.is_file() {
                let env = GherkinEnv::default();
                vec![gherkin::Feature::parse_path(path, env)]
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
                    .collect::<Vec<_>>()
            }
        };

        stream::iter(features().into_iter())
    }
}
