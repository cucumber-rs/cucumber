// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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
#[cfg(feature = "libtest")]
pub mod libtest;
pub mod normalize;
pub mod or;
pub mod out;
pub mod repeat;
pub mod summarize;
pub mod tee;

use async_trait::async_trait;
use sealed::sealed;

use crate::{event, parser, Event};

#[cfg(feature = "output-json")]
#[doc(inline)]
pub use self::json::Json;
#[cfg(feature = "output-junit")]
#[doc(inline)]
pub use self::junit::JUnit;
#[cfg(feature = "libtest")]
#[doc(inline)]
pub use self::libtest::Libtest;
#[doc(inline)]
pub use self::{
    basic::{Basic, Coloring},
    fail_on_skipped::FailOnSkipped,
    normalize::{AssertNormalized, Normalize, Normalized},
    or::Or,
    repeat::Repeat,
    summarize::{Summarizable, Summarize},
    tee::Tee,
};

/// Writer of [`Cucumber`] events to some output.
///
/// As [`Runner`] produces events in a [happened-before] order (see
/// [its order guarantees][1]), [`Writer`]s are required to be [`Normalized`].
///
/// As [`Cucumber::run()`] returns [`Writer`], it can hold some state inside for
/// inspection after execution. See [`Summarize`] and
/// [`Cucumber::run_and_exit()`] for examples.
///
/// [`Cucumber`]: crate::event::Cucumber
/// [`Cucumber::run()`]: crate::Cucumber::run
/// [`Cucumber::run_and_exit()`]: crate::Cucumber::run_and_exit
/// [`Runner`]: crate::Runner
/// [1]: crate::Runner#order-guarantees
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
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
    type Cli: clap::Args;

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

/// [`Writer`] tracking a number of [`Passed`], [`Skipped`], [`Failed`]
/// [`Step`]s and parsing errors.
///
/// [`Failed`]: event::Step::Failed
/// [`Passed`]: event::Step::Passed
/// [`Skipped`]: event::Step::Skipped
/// [`Step`]: gherkin::Step
pub trait Stats<World>: Writer<World> {
    /// Returns number of [`Passed`] [`Step`]s.
    ///
    /// [`Passed`]: event::Step::Passed
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn passed_steps(&self) -> usize;

    /// Returns number of [`Skipped`] [`Step`]s.
    ///
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn skipped_steps(&self) -> usize;

    /// Returns number of [`Failed`] [`Step`]s.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn failed_steps(&self) -> usize;

    /// Returns number of retried [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn retried_steps(&self) -> usize;

    /// Returns number of parsing errors.
    #[must_use]
    fn parsing_errors(&self) -> usize;

    /// Returns number of failed [`Scenario`] hooks.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    fn hook_errors(&self) -> usize;

    /// Indicates whether there were failures/errors during execution.
    #[must_use]
    fn execution_has_failed(&self) -> bool {
        self.failed_steps() > 0
            || self.parsing_errors() > 0
            || self.hook_errors() > 0
    }
}

/// Extension of [`Writer`] allowing its normalization and summarization.
#[sealed]
pub trait Ext: Sized {
    /// Asserts this [`Writer`] being [`Normalized`].
    ///
    /// Technically is no-op, only forcing the [`Writer`] to become
    /// [`Normalized`] despite it actually doesn't represent the one.
    ///
    /// If you need a real normalization, use [`normalized()`] instead.
    ///
    /// > ⚠️ __WARNING__: Should be used only in case you are absolutely sure,
    /// >                 that incoming events will be emitted in a
    /// >                 [`Normalized`] order.
    /// >                 For example, in case [`max_concurrent_scenarios()`][1]
    /// >                 is set to `1`.
    ///
    /// [`normalized()`]: Ext::normalized
    /// [1]: crate::runner::Basic::max_concurrent_scenarios()
    #[must_use]
    fn assert_normalized(self) -> AssertNormalized<Self>;

    /// Wraps this [`Writer`] into a [`Normalize`]d version.
    ///
    /// See [`Normalize`] for more information.
    #[must_use]
    fn normalized<W>(self) -> Normalize<W, Self>;

    /// Wraps this [`Writer`] to print a summary at the end of an output.
    ///
    /// See [`Summarize`] for more information.
    #[must_use]
    fn summarized(self) -> Summarize<Self>;

    /// Wraps this [`Writer`] to fail on [`Skipped`] [`Step`]s if their
    /// [`Scenario`] isn't marked with `@allow.skipped` tag.
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

    /// Wraps this [`Writer`] into a [`discard::Stats`] one, providing a no-op
    /// [`StatsWriter`] implementation returning only `0`.
    ///
    /// Intended to be used for feeding a non-[`StatsWriter`] [`Writer`] into a
    /// [`tee()`], as the later accepts only [`StatsWriter`]s.
    ///
    /// [`tee()`]: Ext::tee
    /// [`StatsWriter`]: Stats
    #[must_use]
    fn discard_stats_writes(self) -> discard::Stats<Self>;
}

#[sealed]
impl<T> Ext for T {
    fn assert_normalized(self) -> AssertNormalized<Self> {
        AssertNormalized::new(self)
    }

    fn normalized<W>(self) -> Normalize<W, Self> {
        Normalize::new(self)
    }

    fn summarized(self) -> Summarize<Self> {
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

    fn discard_stats_writes(self) -> discard::Stats<Self> {
        discard::Stats::wrap(self)
    }
}

/// Marker indicating that a [`Writer`] doesn't transform or rearrange events.
///
/// It's used to ensure that a [`Writer`]s pipeline is built in the right order,
/// avoiding situations like an event transformation isn't done before it's
/// [`Repeat`]ed.
///
/// # Example
///
/// If you want to pipeline [`FailOnSkipped`], [`Summarize`] and [`Repeat`]
/// [`Writer`]s, the code won't compile because of the wrong pipelining order.
///
/// ```rust,compile_fail
/// # use cucumber::{writer, World, WriterExt as _};
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout()
///             .fail_on_skipped() // Fails as `Repeat` will re-output skipped
///             .repeat_failed()   // steps instead of failed ones.
///             .summarized()
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # }
/// ```
///
/// ```rust,compile_fail
/// # use cucumber::{writer, World, WriterExt as _};
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout()
///             .repeat_failed()
///             .fail_on_skipped() // Fails as `Summarize` will count skipped
///             .summarized()      // steps instead of `failed` ones.
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # }
/// ```
///
/// ```rust
/// # use std::panic::AssertUnwindSafe;
/// #
/// # use cucumber::{writer, World, WriterExt as _};
/// # use futures::FutureExt as _;
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// # let fut = async {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout() // And, finally, print them.
///             .repeat_failed()    // Then, repeat failed ones once again.
///             .summarized()       // Only then, count summary for them.
///             .fail_on_skipped(), // First, transform skipped steps to failed.
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # };
/// # let err = AssertUnwindSafe(fut)
/// #     .catch_unwind()
/// #     .await
/// #     .expect_err("should err");
/// # let err = err.downcast_ref::<String>().unwrap();
/// # assert_eq!(err, "1 step failed");
/// # }
/// ```
///
/// [`Failed`]: event::Step::Failed
/// [`Skipped`]: event::Step::Skipped
pub trait NonTransforming {}

/// Standard verbosity levels of a [`Writer`].
#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)]
pub enum Verbosity {
    /// None additional info.
    #[default]
    Default = 0,

    /// Outputs the whole [`World`] on [`Failed`] [`Step`]s whenever is
    /// possible.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    /// [`World`]: crate::World
    ShowWorld = 1,

    /// Additionally to [`Verbosity::ShowWorld`] outputs [Doc Strings].
    ///
    /// [Doc Strings]: https://cucumber.io/docs/gherkin/reference#doc-strings
    ShowWorldAndDocString = 2,
}

impl From<u8> for Verbosity {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Default,
            1 => Self::ShowWorld,
            _ => Self::ShowWorldAndDocString,
        }
    }
}

impl From<Verbosity> for u8 {
    fn from(v: Verbosity) -> Self {
        match v {
            Verbosity::Default => 0,
            Verbosity::ShowWorld => 1,
            Verbosity::ShowWorldAndDocString => 2,
        }
    }
}

impl Verbosity {
    /// Indicates whether [`World`] should be outputted on [`Failed`] [`Step`]s
    /// implying this [`Verbosity`].
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    /// [`World`]: crate::World
    #[must_use]
    pub const fn shows_world(&self) -> bool {
        matches!(self, Self::ShowWorld | Self::ShowWorldAndDocString)
    }

    /// Indicates whether [`Step::docstring`]s should be outputted implying this
    /// [`Verbosity`].
    ///
    /// [`Step::docstring`]: gherkin::Step::docstring
    #[must_use]
    pub const fn shows_docstring(&self) -> bool {
        matches!(self, Self::ShowWorldAndDocString)
    }
}
