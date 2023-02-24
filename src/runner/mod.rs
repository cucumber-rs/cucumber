// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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

use crate::{event, parser, Event};

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
/// following the [happened-before] order). Those rules are applied also to
/// [`Rule`]s and [`Feature`]s. If you want to avoid those interruptions for
/// some [`Scenario`], it should be resolved as [`ScenarioType::Serial`] by the
/// [`Runner`].
///
/// Because of that, [`Writer`], accepting events produced by a [`Runner`] has
/// to be [`Normalized`].
///
/// All those rules are considered in a [`Basic`] reference [`Runner`]
/// implementation.
///
/// [`Cucumber`]: event::Cucumber
/// [`Feature`]: gherkin::Feature
/// [`Normalized`]: crate::writer::Normalized
/// [`Parser`]: crate::Parser
/// [`Rule`]: gherkin::Rule
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
/// [`Writer`]: crate::Writer
///
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
pub trait Runner<World> {
    /// CLI options of this [`Runner`]. In case no options should be introduced,
    /// just use [`cli::Empty`].
    ///
    /// All CLI options from [`Parser`], [`Runner`] and [`Writer`] will be
    /// merged together, so overlapping arguments will cause a runtime panic.
    ///
    /// [`cli::Empty`]: crate::cli::Empty
    /// [`Parser`]: crate::Parser
    /// [`Writer`]: crate::Writer
    type Cli: clap::Args;

    /// Output events [`Stream`].
    type EventStream: Stream<
        Item = parser::Result<Event<event::Cucumber<World>>>,
    >;

    /// Executes the given [`Stream`] of [`Feature`]s transforming it into
    /// a [`Stream`] of executed [`Cucumber`] events.
    ///
    /// [`Cucumber`]: event::Cucumber
    /// [`Feature`]: gherkin::Feature
    fn run<S>(self, features: S, cli: Self::Cli) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static;
}
