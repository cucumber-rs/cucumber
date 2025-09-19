// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Core traits for Cucumber event writers.
//!
//! This module contains the fundamental traits that define how writers handle
//! Cucumber events, provide arbitrary output, and track execution statistics.

use std::future::Future;

use crate::{Event, event, parser};

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
/// [`Normalized`]: super::Normalized
/// [`Runner`]: crate::Runner
/// [`Summarize`]: super::Summarize
/// [1]: crate::Runner#order-guarantees
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
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
    fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<World>>>,
        cli: &Self::Cli,
    ) -> impl Future<Output = ()>;
}

/// [`Writer`] that also can output an arbitrary `Value` in addition to
/// regular [`Cucumber`] events.
///
/// [`Cucumber`]: event::Cucumber
pub trait Arbitrary<World, Value>: Writer<World> {
    /// Writes `val` to the [`Writer`]'s output.
    fn write(&mut self, val: Value) -> impl Future<Output = ()>;
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

    struct MockWriter {
        passed: usize,
        skipped: usize,
        failed: usize,
        retried: usize,
        parsing_errors: usize,
        hook_errors: usize,
    }

    impl Default for MockWriter {
        fn default() -> Self {
            Self {
                passed: 0,
                skipped: 0,
                failed: 0,
                retried: 0,
                parsing_errors: 0,
                hook_errors: 0,
            }
        }
    }

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

    impl Stats<MockWorld> for MockWriter {
        fn passed_steps(&self) -> usize {
            self.passed
        }

        fn skipped_steps(&self) -> usize {
            self.skipped
        }

        fn failed_steps(&self) -> usize {
            self.failed
        }

        fn retried_steps(&self) -> usize {
            self.retried
        }

        fn parsing_errors(&self) -> usize {
            self.parsing_errors
        }

        fn hook_errors(&self) -> usize {
            self.hook_errors
        }
    }

    impl Arbitrary<MockWorld, String> for MockWriter {
        fn write(&mut self, _val: String) -> impl Future<Output = ()> {
            future::ready(())
        }
    }

    #[test]
    fn test_stats_execution_has_failed() {
        let mut writer = MockWriter::default();
        assert!(!writer.execution_has_failed());

        writer.failed = 1;
        assert!(writer.execution_has_failed());

        writer.failed = 0;
        writer.parsing_errors = 1;
        assert!(writer.execution_has_failed());

        writer.parsing_errors = 0;
        writer.hook_errors = 1;
        assert!(writer.execution_has_failed());
    }

    #[test]
    fn test_stats_getters() {
        let writer = MockWriter {
            passed: 5,
            skipped: 3,
            failed: 2,
            retried: 1,
            parsing_errors: 0,
            hook_errors: 0,
        };

        assert_eq!(writer.passed_steps(), 5);
        assert_eq!(writer.skipped_steps(), 3);
        assert_eq!(writer.failed_steps(), 2);
        assert_eq!(writer.retried_steps(), 1);
        assert_eq!(writer.parsing_errors(), 0);
        assert_eq!(writer.hook_errors(), 0);
    }
}