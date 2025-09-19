// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Extension trait for writer functionality.
//!
//! This module provides the [`Ext`] trait that allows normalization,
//! summarization, and various transformations of writers through a
//! fluent interface.

use sealed::sealed;

use crate::{Event, event, parser};
use super::{
    AssertNormalized, FailOnSkipped, Normalize, Repeat, Summarize, Tee,
    discard, Writer,
};

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
    /// [`Normalized`]: super::Normalized
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
    /// [`ArbitraryWriter`]: super::Arbitrary
    #[must_use]
    fn discard_arbitrary_writes(self) -> discard::Arbitrary<Self>;

    /// Wraps this [`Writer`] into a [`discard::Stats`] one, providing a no-op
    /// [`StatsWriter`] implementation returning only `0`.
    ///
    /// Intended to be used for feeding a non-[`StatsWriter`] [`Writer`] into a
    /// [`tee()`], as the later accepts only [`StatsWriter`]s.
    ///
    /// [`tee()`]: Ext::tee
    /// [`StatsWriter`]: super::Stats
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

    fn fail_on_skipped_with<F>(self, with: F) -> FailOnSkipped<Self, F>
    where
        F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> bool,
    {
        FailOnSkipped::with(self, with)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::future;

    #[derive(Debug, Default)]
    struct MockWorld;

    #[derive(Debug, Default)]
    struct MockCli;

    impl clap::FromArgMatches for MockCli {
        fn from_arg_matches(_matches: &clap::ArgMatches) -> clap::error::Result<Self> {
            Ok(Self)
        }
        fn update_from_arg_matches(&mut self, _matches: &clap::ArgMatches) -> clap::error::Result<()> {
            Ok(())
        }
    }

    impl clap::Args for MockCli {
        fn augment_args(cmd: clap::Command) -> clap::Command {
            cmd
        }
        fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
            cmd
        }
    }

    struct MockWriter;

    impl Writer<MockWorld> for MockWriter {
        type Cli = MockCli;

        fn handle_event(
            &mut self,
            _event: parser::Result<Event<event::Cucumber<MockWorld>>>,
            _cli: &Self::Cli,
        ) -> impl Future<Output = ()> {
            future::ready(())
        }
    }

    #[test]
    fn test_ext_trait_methods_exist() {
        let writer = MockWriter;
        
        // Test that all extension methods are available
        let _assert_normalized = writer.assert_normalized();
        
        let writer = MockWriter;
        let _normalized = writer.normalized::<MockWorld>();
        
        let writer = MockWriter;
        let _summarized = writer.summarized();
        
        let writer = MockWriter;
        let _fail_on_skipped = writer.fail_on_skipped();
        
        let writer = MockWriter;
        let _fail_on_skipped_with = writer.fail_on_skipped_with(|_, _, _| false);
        
        let writer = MockWriter;
        let _repeat_skipped = writer.repeat_skipped::<MockWorld>();
        
        let writer = MockWriter;
        let _repeat_failed = writer.repeat_failed::<MockWorld>();
        
        let writer = MockWriter;
        let _repeat_if = writer.repeat_if::<MockWorld, _>(|_| false);
        
        let writer = MockWriter;
        let other = MockWriter;
        let _tee = writer.tee(other);
        
        let writer = MockWriter;
        let _discard_arbitrary = writer.discard_arbitrary_writes();
        
        let writer = MockWriter;
        let _discard_stats = writer.discard_stats_writes();
    }

    #[test] 
    fn test_ext_trait_fluent_chaining() {
        let writer = MockWriter;
        
        // Test that methods can be chained together
        let _chained = writer
            .assert_normalized()
            .summarized()
            .fail_on_skipped()
            .repeat_failed::<MockWorld>()
            .discard_arbitrary_writes()
            .discard_stats_writes();
    }
}