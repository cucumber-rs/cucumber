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
pub mod fail_on_skipped;
pub mod normalized;
pub mod summarized;
pub mod term;

use async_trait::async_trait;
use sealed::sealed;

use crate::{event, parser, World};

#[doc(inline)]
pub use self::{
    basic::Basic, fail_on_skipped::FailOnSkipped, normalized::Normalized,
    summarized::Summarized,
};

/// Writer of [`Cucumber`] events to some output.
///
/// As [`Cucumber::run()`] returns [`Writer`], it can hold some state inside for
/// inspection after execution. See [`Summarized`] and
/// [`Cucumber::run_and_exit()`] for examples.
///
/// [`Cucumber`]: crate::event::Cucumber
/// [`Cucumber::run()`]: crate::Cucumber::run
/// [`Cucumber::run_and_exit()`]: crate::Cucumber::run_and_exit
#[async_trait(?Send)]
pub trait Writer<World> {
    /// Handles the given [`Cucumber`] event.
    ///
    /// [`Cucumber`]: crate::event::Cucumber
    async fn handle_event(
        &mut self,
        ev: parser::Result<event::Cucumber<World>>,
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

/// [`Writer`] tracking a number of [`Failed`] [`Step`]s and parsing [`Error`]s.
///
/// [`Error`]: parser::Error
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

    /// Returns number of parsing [`Error`]s.
    ///
    /// [`Error`]: parser::Error
    #[must_use]
    fn parsing_errors(&self) -> usize;
}

/// Extension of [`Writer`] allowing its normalization and summarization.
#[sealed]
pub trait Ext<W: World>: Writer<W> + Sized {
    /// Wraps this [`Writer`] into a [`Normalized`] version.
    ///
    /// See [`Normalized`] for more information.
    fn normalized(self) -> Normalized<W, Self>;

    /// Wraps this [`Writer`] to print a summary at the end of an output.
    ///
    /// See [`Summarized`] for more information.
    fn summarized(self) -> Summarized<Self>;

    /// Wraps this [`Writer`] to fail on [`Skipped`] [`Step`]s if their
    /// [`Scenario`] isn't marked with `@allow_skipped` tag.
    ///
    /// See [`FailOnSkipped`] for more information.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    fn fail_on_skipped(self) -> FailOnSkipped<Self>;

    /// Wraps this [`Writer`] to fail on [`Skipped`] [`Step`]s if the given
    /// `with` predicate returns `true`.
    ///
    /// See [`FailOnSkipped`] for more information.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    fn fail_on_skipped_with<F>(self, with: F) -> FailOnSkipped<Self, F>
    where
        F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> bool;
}

#[sealed]
impl<W, T> Ext<W> for T
where
    W: World,
    T: Writer<W> + Sized,
{
    fn normalized(self) -> Normalized<W, Self> {
        Normalized::new(self)
    }

    fn summarized(self) -> Summarized<Self> {
        Summarized::from(self)
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
}
