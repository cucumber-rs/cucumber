// Copyright (c) 2018-2026  Brendan Molloy <brendan@bbqsrc.net>,
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

use std::sync::Arc;

use derive_more::with_trait::{Display, Error as StdError};
use futures::Stream;

#[doc(inline)]
pub use self::basic::Basic;
use crate::feature::ExpandExamplesError;

/// Source of parsed [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
pub trait Parser<I> {
    /// CLI options of this [`Parser`]. In case no options should be introduced,
    /// just use [`cli::Empty`].
    ///
    /// All CLI options from [`Parser`], [`Runner`] and [`Writer`] will be
    /// merged together, so overlapping arguments will cause a runtime panic.
    ///
    /// [`cli::Empty`]: crate::cli::Empty
    /// [`Runner`]: crate::Runner
    /// [`Writer`]: crate::Writer
    type Cli: clap::Args;

    /// Output [`Stream`] of parsed [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    type Output: Stream<Item = Result<gherkin::Feature>> + 'static;

    /// Parses the given `input` into a [`Stream`] of [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    fn parse(self, input: I, cli: Self::Cli) -> Self::Output;
}

/// Result of parsing [Gherkin] files.
///
/// [Gherkin]: https://cucumber.io/docs/gherkin/reference
#[expect(clippy::absolute_paths, reason = "one liner")]
pub type Result<T> = std::result::Result<T, Error>;

/// [`Parser`] error.
#[derive(Clone, Debug, Display, StdError)]
pub enum Error {
    /// Failed to parse a [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    #[display("Failed to parse feature: {_0}")]
    Parsing(Arc<gherkin::ParseFileError>),

    /// Failed to expand [`Examples`]
    ///
    /// [`Examples`]: gherkin::Examples
    #[display("Failed to expand examples: {_0}")]
    ExampleExpansion(Arc<ExpandExamplesError>),
}

impl From<gherkin::ParseFileError> for Error {
    fn from(e: gherkin::ParseFileError) -> Self {
        Self::Parsing(Arc::new(e))
    }
}

impl From<ExpandExamplesError> for Error {
    fn from(e: ExpandExamplesError) -> Self {
        Self::ExampleExpansion(Arc::new(e))
    }
}
