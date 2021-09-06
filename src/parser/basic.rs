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

use std::{
    path::{Path, PathBuf},
    vec,
};

use derive_more::{Display, Error};
use futures::stream;
use gherkin::GherkinEnv;
use globwalk::GlobWalkerBuilder;

use super::{Parser, Result as ParseResult};

/// Default [`Parser`].
///
/// As there is no async runtime-agnostic way to interact with IO, this
/// [`Parser`] is blocking.
#[derive(Clone, Debug, Default)]
pub struct Basic {
    /// Optional custom language of [`gherkin`] keywords.
    ///
    /// Default is English.
    language: Option<String>,
}

impl<I: AsRef<Path>> Parser<I> for Basic {
    type Output = stream::Iter<vec::IntoIter<ParseResult<gherkin::Feature>>>;

    fn parse(self, path: I) -> Self::Output {
        let features = || {
            let path = path.as_ref();
            let path = match path.canonicalize().or_else(|_| {
                let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                buf.push(
                    path.strip_prefix("/")
                        .or_else(|_| path.strip_prefix("./"))
                        .unwrap_or(path),
                );
                buf.as_path().canonicalize()
            }) {
                Ok(p) => p,
                Err(err) => {
                    return vec![Err(gherkin::ParseFileError::Reading {
                        path: path.to_path_buf(),
                        source: err,
                    })];
                }
            };

            if path.is_file() {
                let env = self
                    .language
                    .as_ref()
                    .and_then(|l| GherkinEnv::new(l).ok())
                    .unwrap_or_default();
                vec![gherkin::Feature::parse_path(path, env)]
            } else {
                let walker = GlobWalkerBuilder::new(path, "*.feature")
                    .case_insensitive(true)
                    .build()
                    .unwrap();
                walker
                    .filter_map(Result::ok)
                    .map(|entry| {
                        let env = self
                            .language
                            .as_ref()
                            .and_then(|l| GherkinEnv::new(l).ok())
                            .unwrap_or_default();
                        gherkin::Feature::parse_path(entry.path(), env)
                    })
                    .collect::<Vec<_>>()
            }
        };

        stream::iter(features().into_iter())
    }
}

impl Basic {
    /// Creates a new [`Basic`] [`Parser`].
    #[must_use]
    pub fn new() -> Self {
        Self { language: None }
    }

    /// Sets the provided language to parse [`gherkin`] files in instead of the
    /// default one (English).
    ///
    /// # Errors
    ///
    /// If the provided language isn't supported.
    pub fn language(
        mut self,
        name: String,
    ) -> Result<Self, UnsupportedLanguageError> {
        if !gherkin::is_language_supported(&name) {
            return Err(UnsupportedLanguageError(name));
        }
        self.language = Some(name);
        Ok(self)
    }
}

/// Error of [`gherkin`] not supporting keywords in some language.
#[derive(Debug, Display, Error)]
#[display(fmt = "Language {} isn't supported", _0)]
pub struct UnsupportedLanguageError(#[error(not(source))] pub String);
