// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for outputting [`Cucumber`] events.
//!
//! [`Cucumber`]: crate::event::Cucumber

pub mod basic;
pub mod discard;
pub mod fail_on_skipped;
#[cfg(feature = "output-json")]
pub mod json;
#[cfg(feature = "output-junit")]
pub mod junit;
pub mod normalize;
pub mod out;
pub mod repeat;
pub mod summarize;
pub mod tee;

use async_trait::async_trait;
use sealed::sealed;
use structopt::StructOptInternal;

use crate::{event, parser, Event};

#[cfg(feature = "output-json")]
#[doc(inline)]
pub use self::json::Json;
#[cfg(feature = "output-junit")]
#[doc(inline)]
pub use self::junit::JUnit;
#[doc(inline)]
pub use self::{
    basic::{Basic, Coloring},
    fail_on_skipped::FailOnSkipped,
    normalize::{Normalize, Normalized},
    repeat::Repeat,
    summarize::Summarize,
    tee::Tee,
};

/// Writer of [`Cucumber`] events to some output.
///
/// As [`Runner`] produces events in [happens-before] order (see
/// [Order guarantees]), [`Writer`]s are required to be [`Normalized`].
///
/// As [`Cucumber::run()`] returns [`Writer`], it can hold some state inside for
/// inspection after execution. See [`Summarize`] and
/// [`Cucumber::run_and_exit()`] for examples.
///
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
/// [Order guarantees]: crate::Runner#order-guarantees
/// [`Cucumber`]: crate::event::Cucumber
/// [`Cucumber::run()`]: crate::Cucumber::run
/// [`Cucumber::run_and_exit()`]: crate::Cucumber::run_and_exit
/// [`Runner`]: crate::Runner
#[async_trait(?Send)]
pub trait Writer<World> {
    /// CLI options of this [`Writer`]. In case no options should be introduced,
    /// just use [`cli::Empty`].
    ///
    /// All CLI options from [`Parser`], [`Runner`] and [`Writer`] will be
    /// merged together, so overlapping arguments will cause a runtime panic.
    ///
    /// [`cli::Empty`]: crate::cli::Empty
    /// [`Parser`]: crate::Parser
    /// [`Runner`]: crate::Runner
    /// [`StructOpt`]: structopt::StructOpt
    // We do use `StructOptInternal` here only because `StructOpt::from_args()`
    // requires exactly this trait bound. We don't touch any `StructOptInternal`
    // details being a subject of instability.
    type Cli: StructOptInternal;

    /// Handles the given [`Cucumber`] event.
    ///
    /// [`Cucumber`]: crate::event::Cucumber
    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<World>>>,
        cli: &Self::Cli,
    );
}

/// [`Writer`] that also can output an arbitrary `Value` in addition to
/// regular [`Cucumber`] events.
///
/// [`Cucumber`]: crate::event::Cucumber
#[async_trait(?Send)]
pub trait Arbitrary<'val, World, Value: 'val>: Writer<World> {
    /// Writes `val` to the [`Writer`]'s output.
    async fn write(&mut self, val: Value)
    where
        'val: 'async_trait;
}

/// [`Writer`] tracking a number of [`Failed`] [`Step`]s and parsing errors.
///
/// [`Failed`]: event::Step::Failed
/// [`Step`]: gherkin::Step
pub trait Failure<World>: Writer<World> {
    /// Indicates whether there were failures/errors during execution.
    #[must_use]
    fn execution_has_failed(&self) -> bool {
        self.failed_steps() > 0 || self.parsing_errors() > 0
    }

    /// Returns number of [`Failed`] [`Step`]s.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn failed_steps(&self) -> usize;

    /// Returns number of parsing errors.
    #[must_use]
    fn parsing_errors(&self) -> usize;

    /// Returns number of failed [`Scenario`] hooks.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    fn hook_errors(&self) -> usize;
}

/// Marker trait indicating that [`Writer`] events can be [`Repeat`]ed.
///
/// Most of the [`Writer`]s implement it. Counterexample may be
/// [`FailOnSkipped`], which transforms [`Skipped`] events into [`Failed`].
/// We should [`Repeat`] first and only then [`FailOnSkipped`]. Applying them in
/// reversed order will cause event transformation only on main output, while
/// [`Repeat`] will print untransformed [`Skipped`] events.
///
/// [`Failed`]: event::Step::Failed
/// [`FailOnSkipped`]: crate::writer::FailOnSkipped
/// [`Skipped`]: event::Step::Skipped
pub trait NotTransformEvents {}

/// Extension of [`Writer`] allowing its normalization and summarization.
#[sealed]
pub trait Ext: Sized {
    /// Wraps this [`Writer`] into a [`Normalize`] version.
    ///
    /// See [`Normalize`] for more information.
    #[must_use]
    fn normalize<W>(self) -> Normalize<W, Self>;

    /// Wraps this [`Writer`] to print a summary at the end of an output.
    ///
    /// See [`Summarize`] for more information.
    #[must_use]
    fn summarize(self) -> Summarize<Self>;

    /// Wraps this [`Writer`] to fail on [`Skipped`] [`Step`]s if their
    /// [`Scenario`] isn't marked with `@allow_skipped` tag.
    ///
    /// See [`FailOnSkipped`] for more information.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn fail_on_skipped(self) -> FailOnSkipped<Self>;

    /// Wraps this [`Writer`] to fail on [`Skipped`] [`Step`]s if the given
    /// `with` predicate returns `true`.
    ///
    /// See [`FailOnSkipped`] for more information.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn fail_on_skipped_with<F>(self, with: F) -> FailOnSkipped<Self, F>
    where
        F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> bool;

    /// Wraps this [`Writer`] to re-output [`Skipped`] [`Step`]s at the end of
    /// an output.
    ///
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn repeat_skipped<W>(self) -> Repeat<W, Self>;

    /// Wraps this [`Writer`] to re-output [`Failed`] [`Step`]s or [`Parser`]
    /// errors at the end of an output.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Parser`]: crate::Parser
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn repeat_failed<W>(self) -> Repeat<W, Self>;

    /// Wraps this [`Writer`] to re-output `filter`ed events at the end of an
    /// output.
    #[must_use]
    fn repeat_if<W, F>(self, filter: F) -> Repeat<W, Self, F>
    where
        F: Fn(&parser::Result<Event<event::Cucumber<W>>>) -> bool;

    /// Attaches the provided `other` [`Writer`] to the current one for passing
    /// events to both of them simultaneously.
    #[must_use]
    fn tee<W, Wr: Writer<W>>(self, other: Wr) -> Tee<Self, Wr>;

    /// Wraps this [`Writer`] into a [`discard::Arbitrary`] one, providing a
    /// no-op [`ArbitraryWriter`] implementation.
    ///
    /// Intended to be used for feeding a non-[`ArbitraryWriter`] [`Writer`]
    /// into a [`tee()`], as the later accepts only [`ArbitraryWriter`]s.
    ///
    /// [`tee()`]: Ext::tee
    /// [`ArbitraryWriter`]: Arbitrary
    #[must_use]
    fn discard_arbitrary_writes(self) -> discard::Arbitrary<Self>;

    /// Wraps this [`Writer`] into a [`discard::Arbitrary`] one, providing a
    /// no-op [`FailureWriter`] implementation returning only `0`.
    ///
    /// Intended to be used for feeding a non-[`FailureWriter`] [`Writer`]
    /// into a [`tee()`], as the later accepts only [`FailureWriter`]s.
    ///
    /// [`tee()`]: Ext::tee
    /// [`FailureWriter`]: Failure
    #[must_use]
    fn discard_failure_writes(self) -> discard::Failure<Self>;
}

#[sealed]
impl<T> Ext for T {
    fn normalize<W>(self) -> Normalize<W, Self> {
        Normalize::new(self)
    }

    fn summarize(self) -> Summarize<Self> {
        Summarize::from(self)
    }

    fn fail_on_skipped(self) -> FailOnSkipped<Self> {
        FailOnSkipped::from(self)
    }

    fn fail_on_skipped_with<F>(self, f: F) -> FailOnSkipped<Self, F>
    where
        F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> bool,
    {
        FailOnSkipped::with(self, f)
    }

    fn repeat_skipped<W>(self) -> Repeat<W, Self> {
        Repeat::skipped(self)
    }

    fn repeat_failed<W>(self) -> Repeat<W, Self> {
        Repeat::failed(self)
    }

    fn repeat_if<W, F>(self, filter: F) -> Repeat<W, Self, F>
    where
        F: Fn(&parser::Result<Event<event::Cucumber<W>>>) -> bool,
    {
        Repeat::new(self, filter)
    }

    fn tee<W, Wr: Writer<W>>(self, other: Wr) -> Tee<Self, Wr> {
        Tee::new(self, other)
    }

    fn discard_arbitrary_writes(self) -> discard::Arbitrary<Self> {
        discard::Arbitrary::wrap(self)
    }

    fn discard_failure_writes(self) -> discard::Failure<Self> {
        discard::Failure::wrap(self)
    }
}
