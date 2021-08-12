//! Tools for parsing [Gherkin] files.
//!
//! [Gherkin]: https://cucumber.io/docs/gherkin/reference

pub mod basic;

use futures::Stream;

#[doc(inline)]
pub use basic::Basic;

/// Trait for sourcing parsed [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
pub trait Parser<I> {
    /// Output [`Stream`] of parsed [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    type Output: Stream<Item = gherkin::Feature> + 'static;

    /// Parses the given `input` into [`Stream`] of [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    fn parse(self, input: I) -> Self::Output;
}
