// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for parsing [Gherkin] files.
//!
//! [Gherkin]: https://cucumber.io/docs/gherkin/reference

pub mod basic;

use std::{io, path::PathBuf};

use derive_more::{Display, Error, From};
use futures::Stream;

#[doc(inline)]
pub use self::basic::Basic;

/// Source of parsed [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
pub trait Parser<I> {
    /// Output [`Stream`] of parsed [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    type Output: Stream<Item = Result<gherkin::Feature>> + 'static;

    /// Parses the given `input` into a [`Stream`] of [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    fn parse(self, input: I) -> Self::Output;
}

/// Result of parsing [Gherkin] files.
///
/// [Gherkin]: https://cucumber.io/docs/gherkin/reference
pub type Result<T> = std::result::Result<T, Error>;

/// Error while parsing [Gherkin] file.
///
/// [Gherkin]: https://cucumber.io/docs/gherkin/reference
#[derive(Debug, Display, Error, From)]
pub enum Error {
    /// Unsupported language.
    #[display(fmt = "Language error: {}", _0)]
    Language(gherkin::EnvError),

    /// Parsing file error.
    #[display(fmt = "Parsing error: {}, file: {}", source, "path.display()")]
    Parse {
        /// Original error of parsing.
        source: gherkin::ParseError,

        /// Path of the file.
        path: PathBuf,
    },

    /// Reading file error.
    #[display(fmt = "Reading error: {}, file: {}", source, "path.display()")]
    Read {
        /// Original reading error.
        source: io::Error,

        /// Path of the file.
        path: PathBuf,
    },
}
