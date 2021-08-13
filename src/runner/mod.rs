// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for executing [`Step`]s.
//!
//! [`Step`]: crate::Step
//!
//! [Gherkin]: https://cucumber.io/docs/gherkin/reference/

pub mod basic;

use futures::Stream;

use crate::event;

#[doc(inline)]
pub use basic::{Basic, ScenarioType};

/// Trait for sourcing [`Cucumber`] events from parsed [Gherkin] files.
///
/// # Events order guarantees
///
/// As [`Scenario`]s can be executed concurrently, there are no strict order
/// guarantees. But implementors are expected to source events in such order,
/// that by rearranging __only__ sub-elements ([`Scenario`]s in a particular
/// [`Feature`], etc...) we can restore original [`Parser`] order.
///
/// Note, that those rules are recommended in case you are using
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
