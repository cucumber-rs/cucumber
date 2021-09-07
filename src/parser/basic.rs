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
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    vec,
};

use derive_more::{Display, Error};
use futures::stream;
use gherkin::GherkinEnv;
use globwalk::GlobWalkerBuilder;
use once_cell::sync::Lazy;
use regex::Regex;

use super::{Error as ParseError, Parser, Result as ParseResult};

/// Default [`Parser`].
///
/// As there is no async runtime-agnostic way to interact with IO, this
/// [`Parser`] is blocking.
#[derive(Clone, Debug, Default)]
pub struct Basic {
    /// Optional custom language of [`gherkin`] keywords.
    ///
    /// Default is English.
    language: Option<Cow<'static, str>>,
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
                Err(source) => {
                    return vec![Err(ParseError::Reading {
                        path: path.to_path_buf(),
                        source,
                    })];
                }
            };

            let parse_feature = |path: &Path| {
                fs::read_to_string(path)
                    .map_err(|source| ParseError::Reading {
                        path: path.to_path_buf(),
                        source,
                    })
                    .and_then(|f| {
                        self.get_language(&f)
                            .map(|l| (l, f))
                            .map_err(Into::into)
                    })
                    .and_then(|(l, file)| {
                        gherkin::Feature::parse(file, l)
                            .map(|mut f| {
                                f.path = Some(path.to_path_buf());
                                f
                            })
                            .map_err(|source| ParseError::Parse {
                                source,
                                path: path.to_path_buf(),
                            })
                    })
            };

            if path.is_file() {
                vec![parse_feature(&path)]
            } else {
                let walker = GlobWalkerBuilder::new(path, "*.feature")
                    .case_insensitive(true)
                    .build()
                    .unwrap();
                walker
                    .filter_map(Result::ok)
                    .map(|entry| parse_feature(entry.path()))
                    .collect()
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
        name: impl Into<Cow<'static, str>>,
    ) -> Result<Self, UnsupportedLanguageError> {
        let name = name.into();
        if !gherkin::is_language_supported(&name) {
            return Err(UnsupportedLanguageError(name));
        }
        self.language = Some(name);
        Ok(self)
    }

    /// Returns language to parse with `file`.
    ///
    /// 1. If `# language: ` comment present, use it;
    /// 2. If default language was set with [`Self::language()`], use it;
    /// 3. If none of the above, assume english.
    fn get_language(
        &self,
        file: impl AsRef<str>,
    ) -> Result<GherkinEnv, gherkin::EnvError> {
        static RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"# language: ([^\r\n]+)").unwrap());

        let lang = || Some(RE.captures(file.as_ref())?.get(1)?.as_str());

        lang().map_or_else(
            || {
                Ok(self
                    .language
                    .as_ref()
                    .and_then(|l| GherkinEnv::new(l).ok())
                    .unwrap_or_default())
            },
            |l| GherkinEnv::new(l),
        )
    }
}

/// Error of [`gherkin`] not supporting keywords in some language.
#[derive(Debug, Display, Error)]
#[display(fmt = "Language {} isn't supported", _0)]
pub struct UnsupportedLanguageError(
    #[error(not(source))] pub Cow<'static, str>,
);
