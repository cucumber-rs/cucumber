//! Tools for executing [`Step`]s on parsed [Gherkin] files.
//!
//! [`Step`]: crate::Step
//!
//! [Gherkin]: https://cucumber.io/docs/gherkin/reference/

pub mod basic;

use futures::Stream;

use crate::event;

pub use basic::Basic;

/// Trait for sourcing [`Cucumber`] events from parsed [Gherkin] files.
///
/// # Events order guarantees
///
/// As [`Scenario`]s can be executed concurrently, there are no strict order
/// guarantees. But implementors are expected to source events in such order,
/// that by rearranging __only__ sub-elements ([`Scenario`]s in a particular
/// [`Feature`], etc...) we can restore original [`Parser`] order.
///
/// Note that those rules are recommended in case you are using
/// [`writer::Normalized`]. Strictly speaking no one is stopping you from
/// implementing [`Runner`] which sources events completely out-of-order.
///
/// [`Cucumber`]: event::Cucumber
/// [`Feature`]: gherkin::Feature
/// [`writer::Normalized`]: crate::writer::Normalized
/// [`Parser`]: crate::Parser
/// [`Scenario`]: gherkin::Scenario
///
/// [Gherkin]: https://cucumber.io/docs/gherkin/reference/
pub trait Runner<World> {
    /// Output events [`Stream`].
    type EventStream: Stream<Item = event::Cucumber<World>>;

    /// Transforms incoming [`Feature`]s [`Stream`] into [`Self::EventStream`].
    ///
    /// [`Feature`]: gherkin::Feature
    fn run<S>(self, features: S) -> Self::EventStream
    where
        S: Stream<Item = gherkin::Feature> + 'static;
}
