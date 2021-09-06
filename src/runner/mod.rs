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

use crate::{event, parser};

#[doc(inline)]
pub use self::basic::{Basic, ScenarioType};

/// Executor of [`Parser`] output producing [`Cucumber`] events for [`Writer`].
///
/// # Order guarantees
///
/// Implementors are expected to source events in a [happened-before] order. For
/// example [`event::Scenario::Started`] for a single [`Scenario`] should
/// predate any other events of this [`Scenario`], while
/// [`event::Scenario::Finished`] should be the last one. [`Step`] events of
/// this [`Scenario`] should be emitted in order of declaration in `.feature`
/// file. But as [`Scenario`]s can be executed concurrently, events from one
/// [`Scenario`] can be interrupted by events of a different one (which are also
/// follow the [happened-before] order). Those rules are applied also to
/// [`Rule`]s and [`Feature`]s. If you want to avoid those interruptions for
/// some [`Scenario`], it should be resolved as [`ScenarioType::Serial`] by the
/// [`Runner`].
///
/// All those rules are considered in a [`Basic`] reference [`Runner`]
/// implementation.
///
/// Note, that those rules are recommended in case you are using a
/// [`writer::Normalized`]. Strictly speaking, no one is stopping you from
/// implementing [`Runner`] which sources events completely out-of-order or even
/// skips some of them. For example, this can be useful if you care only about
/// failed [`Step`]s.
///
/// [`Cucumber`]: event::Cucumber
/// [`Feature`]: gherkin::Feature
/// [`Parser`]: crate::Parser
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
/// [`Writer`]: crate::Writer
/// [`writer::Normalized`]: crate::writer::Normalized
///
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
pub trait Runner<World> {
    /// Output events [`Stream`].
    type EventStream: Stream<Item = event::Cucumber<World>>;

    /// Executes the given [`Stream`] of [`Feature`]s transforming it into
    /// a [`Stream`] of executed [`Cucumber`] events.
    ///
    /// [`Cucumber`]: event::Cucumber
    /// [`Feature`]: gherkin::Feature
    fn run<S>(self, features: S) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static;
}
