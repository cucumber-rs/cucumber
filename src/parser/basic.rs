// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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
    path::{Path, PathBuf},
    str::FromStr,
    vec,
};

use derive_more::{Display, Error};
use futures::stream;
use gherkin::GherkinEnv;
use globwalk::{GlobWalker, GlobWalkerBuilder};
use itertools::Itertools as _;

use crate::feature::Ext as _;

use super::{Error as ParseError, Parser};

/// CLI options of a [`Basic`] [`Parser`].
#[derive(clap::Args, Clone, Debug, Default)]
#[group(skip)]
pub struct Cli {
    /// Glob pattern to look for feature files with. By default, looks for
    /// `*.feature`s in the path configured tests runner.
    #[arg(
        id = "input",
        long = "input",
        short = 'i',
        value_name = "glob",
        global = true
    )]
    pub features: Option<Walker>,
}

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
    type Cli = Cli;

    type Output =
        stream::Iter<vec::IntoIter<Result<gherkin::Feature, ParseError>>>;

    fn parse(self, path: I, cli: Self::Cli) -> Self::Output {
        let walk = |walker: GlobWalker| {
            walker
                .filter_map(Result::ok)
                .sorted_by(|l, r| Ord::cmp(l.path(), r.path()))
                .map(|file| {
                    let env = self
                        .language
                        .as_ref()
                        .and_then(|l| GherkinEnv::new(l).ok())
                        .unwrap_or_default();
                    gherkin::Feature::parse_path(file.path(), env)
                })
                .collect::<Vec<_>>()
        };

        let get_features_path = || {
            let path = path.as_ref();
            path.canonicalize()
                .or_else(|_| {
                    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                    buf.push(
                        path.strip_prefix("/")
                            .or_else(|_| path.strip_prefix("./"))
                            .unwrap_or(path),
                    );
                    buf.as_path().canonicalize()
                })
                .map_err(|e| gherkin::ParseFileError::Reading {
                    path: path.to_path_buf(),
                    source: e,
                })
        };

        let features = || {
            let features = if let Some(walker) = cli.features {
                walk(globwalk::glob(walker.0).unwrap_or_else(|e| {
                    unreachable!("Invalid glob pattern: {e}")
                }))
            } else {
                let feats_path = match get_features_path() {
                    Ok(p) => p,
                    Err(e) => return vec![Err(e.into())],
                };

                if feats_path.is_file() {
                    let env = self
                        .language
                        .as_ref()
                        .and_then(|l| GherkinEnv::new(l).ok())
                        .unwrap_or_default();
                    vec![gherkin::Feature::parse_path(feats_path, env)]
                } else {
                    let w = GlobWalkerBuilder::new(feats_path, "*.feature")
                        .case_insensitive(true)
                        .build()
                        .unwrap_or_else(|e| {
                            unreachable!("`GlobWalkerBuilder` panicked: {e}")
                        });
                    walk(w)
                }
            };

            features
                .into_iter()
                .map(|f| match f {
                    Ok(f) => f.expand_examples().map_err(ParseError::from),
                    Err(e) => Err(e.into()),
                })
                .collect()
        };

        stream::iter(features())
    }
}

impl Basic {
    /// Creates a new [`Basic`] [`Parser`].
    #[must_use]
    pub const fn new() -> Self {
        Self { language: None }
    }

    /// Sets the provided language to parse [`gherkin`] files with instead of
    /// the default one (English).
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
}

/// Error of [`gherkin`] not supporting keywords in some language.
#[derive(Clone, Debug, Display, Error)]
#[display(fmt = "Language {} isn't supported", _0)]
pub struct UnsupportedLanguageError(
    #[error(not(source))] pub Cow<'static, str>,
);

/// Wrapper over [`GlobWalker`] implementing a [`FromStr`].
#[derive(Clone, Debug)]
pub struct Walker(String);

impl FromStr for Walker {
    type Err = globwalk::GlobError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        globwalk::glob(s).map(|_| Self(s.to_owned()))
    }
}
