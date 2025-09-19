// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Core libtest writer structure and implementation.

use std::{
    fmt::Debug,
    io, mem,
    time::SystemTime,
};

use crate::{
    Event, World, Writer, cli,
    event,
    parser,
    writer::{
        self, Normalize,
        common::{WriterStats, OutputFormatter},
    },
};

use super::cli::{Cli, Format};

/// [`libtest`][1] compatible [`Writer`].
///
/// Currently used only to support `--format=json` option.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into a
/// [`writer::Normalize`], otherwise will produce output [`Event`]s in a broken
/// order.
///
/// Ideally, we shouldn't wrap this into a [`writer::Normalize`] and leave this
/// to tools, parsing JSON output. Unfortunately, not all tools can do that (ex.
/// [`IntelliJ Rust`][2]), so it's still recommended to wrap this into
/// [`writer::Normalize`] even if it can mess up timing reports.
///
/// [`Normalized`]: writer::Normalized
/// [1]: https://doc.rust-lang.org/rustc/tests/index.html
/// [2]: https://github.com/intellij-rust/intellij-rust/issues/9041
#[derive(Debug)]
pub struct Libtest<W, Out: io::Write = io::Stdout> {
    /// [`io::Write`] implementor to output into.
    pub(super) output: Out,

    /// Collection of events before [`ParsingFinished`] is received.
    ///
    /// Until a [`ParsingFinished`] is received, all the events are stored
    /// inside [`Libtest::events`] and outputted only after that event is
    /// received. This is done, because [`libtest`][1]'s first event must
    /// contain number of executed test cases.
    ///
    /// [`ParsingFinished`]: event::Cucumber::ParsingFinished
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    pub(super) events: Vec<parser::Result<Event<event::Cucumber<W>>>>,

    /// Indicates whether a [`ParsingFinished`] event was received.
    ///
    /// [`ParsingFinished`]: event::Cucumber::ParsingFinished
    pub(super) parsed_all: bool,

    /// Number of passed [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub(super) passed: usize,

    /// Number of failed [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub(super) failed: usize,

    /// Number of retried [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub(super) retried: usize,

    /// Number of skipped [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub(super) ignored: usize,

    /// Number of [`Parser`] errors.
    ///
    /// [`Parser`]: crate::Parser
    pub(super) parsing_errors: usize,

    /// Number of [`Hook`] errors.
    ///
    /// [`Hook`]: event::Hook
    pub(super) hook_errors: usize,

    /// Number of [`Feature`]s with [`path`] set to [`None`].
    ///
    /// This value is used to generate a unique name for each [`Feature`] to
    /// avoid name collisions.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`path`]: gherkin::Feature::path
    pub(super) features_without_path: usize,

    /// [`SystemTime`] when the [`Started`] event was received.
    ///
    /// [`Started`]: event::Cucumber::Started
    pub(super) started_at: Option<SystemTime>,

    /// [`SystemTime`] when the [`Step::Started`]/[`Hook::Started`] event was
    /// received.
    ///
    /// [`Hook::Started`]: event::Hook::Started
    /// [`Step::Started`]: event::Step::Started
    pub(super) step_started_at: Option<SystemTime>,

    /// Consolidated statistics tracking.
    pub(super) stats: WriterStats,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World, Out: Clone + io::Write> Clone for Libtest<World, Out> {
    fn clone(&self) -> Self {
        Self {
            output: self.output.clone(),
            events: self.events.clone(),
            parsed_all: self.parsed_all,
            passed: self.passed,
            failed: self.failed,
            retried: self.retried,
            ignored: self.ignored,
            parsing_errors: self.parsing_errors,
            hook_errors: self.hook_errors,
            features_without_path: self.features_without_path,
            started_at: self.started_at,
            step_started_at: self.step_started_at,
            stats: self.stats.clone(),
        }
    }
}

impl<W: World + Debug, Out: io::Write> Writer<W> for Libtest<W, Out> {
    type Cli = Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.handle_cucumber_event(event, cli);
    }
}

/// Shortcut of a [`Libtest::or()`] return type.
pub type Or<W, Wr> = writer::Or<
    Wr,
    Normalize<W, Libtest<W, io::Stdout>>,
    fn(
        &parser::Result<Event<event::Cucumber<W>>>,
        &cli::Compose<<Wr as Writer<W>>::Cli, Cli>,
    ) -> bool,
>;

/// Shortcut of a [`Libtest::or_basic()`] return type.
pub type OrBasic<W> = Or<W, writer::Summarize<Normalize<W, writer::Basic>>>;

impl<W: Debug + World> Libtest<W, io::Stdout> {
    /// Creates a new [`Normalized`] [`Libtest`] [`Writer`] outputting into the
    /// [`io::Stdout`].
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn stdout() -> Normalize<W, Self> {
        Self::new(io::stdout())
    }

    /// Creates a new [`Writer`] which uses a [`Normalized`] [`Libtest`] in case
    /// [`Cli::format`] is set to [`Json`], or provided the `writer` otherwise.
    ///
    /// [`Json`]: Format::Json
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn or<AnotherWriter: Writer<W>>(
        writer: AnotherWriter,
    ) -> Or<W, AnotherWriter> {
        writer::Or::new(writer, Self::stdout(), |_, cli| {
            !matches!(cli.right.format, Some(Format::Json))
        })
    }

    /// Creates a new [`Writer`] which uses a [`Normalized`] [`Libtest`] in case
    /// [`Cli::format`] is set to [`Json`], or a [`Normalized`]
    /// [`writer::Basic`] otherwise.
    ///
    /// [`Json`]: Format::Json
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn or_basic() -> OrBasic<W> {
        Self::or(writer::Basic::stdout().summarized())
    }
}

impl<W: Debug + World, Out: io::Write> Libtest<W, Out> {
    /// Creates a new [`Normalized`] [`Libtest`] [`Writer`] outputting into the
    /// provided `output`.
    ///
    /// Theoretically, normalization should be done by the tool that's consuming
    /// the output og this [`Writer`]. But lack of clear specification of the
    /// [`libtest`][1]'s JSON output leads to some tools [struggling][2] to
    /// interpret it. So, we recommend using a [`Normalized`] [`Libtest::new()`]
    /// rather than a non-[`Normalized`] [`Libtest::raw()`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    /// [2]: https://github.com/intellij-rust/intellij-rust/issues/9041
    #[must_use]
    pub fn new(output: Out) -> Normalize<W, Self> {
        Self::raw(output).normalized()
    }

    /// Creates a new non-[`Normalized`] [`Libtest`] [`Writer`] outputting into
    /// the provided `output`.
    ///
    /// Theoretically, normalization should be done by the tool that's consuming
    /// the output og this [`Writer`]. But lack of clear specification of the
    /// [`libtest`][1]'s JSON output leads to some tools [struggling][2] to
    /// interpret it. So, we recommend using a [`Normalized`] [`Libtest::new()`]
    /// rather than a non-[`Normalized`] [`Libtest::raw()`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://doc.rust-lang.org/rustc/tests/index.html
    /// [2]: https://github.com/intellij-rust/intellij-rust/issues/9041
    #[must_use]
    pub const fn raw(output: Out) -> Self {
        Self {
            output,
            events: Vec::new(),
            parsed_all: false,
            passed: 0,
            failed: 0,
            retried: 0,
            parsing_errors: 0,
            hook_errors: 0,
            ignored: 0,
            features_without_path: 0,
            started_at: None,
            step_started_at: None,
            stats: WriterStats::new(),
        }
    }
}

impl<W, O: io::Write> writer::NonTransforming for Libtest<W, O> {}

impl<W, O> writer::Stats<W> for Libtest<W, O>
where
    O: io::Write,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        self.passed
    }

    fn skipped_steps(&self) -> usize {
        self.ignored
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

impl<W, Val, Out> writer::Arbitrary<W, Val> for Libtest<W, Out>
where
    W: World + Debug,
    Val: AsRef<str>,
    Out: io::Write,
{
    async fn write(&mut self, val: Val) {
        self.output
            .write_line(val.as_ref())
            .unwrap_or_else(|e| panic!("failed to write: {e}"));
    }
}

impl<W, Out: io::Write> OutputFormatter for Libtest<W, Out> {
    type Output = Out;

    fn output_mut(&mut self) -> &mut Self::Output {
        &mut self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[derive(Debug)]
    struct MockWorld;
    impl World for MockWorld {}

    mod libtest_struct_tests {
        use super::*;

        #[test]
        fn libtest_raw_creates_default_state() {
            let writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            
            assert_eq!(writer.events.len(), 0);
            assert!(!writer.parsed_all);
            assert_eq!(writer.passed, 0);
            assert_eq!(writer.failed, 0);
            assert_eq!(writer.retried, 0);
            assert_eq!(writer.ignored, 0);
            assert_eq!(writer.parsing_errors, 0);
            assert_eq!(writer.hook_errors, 0);
            assert_eq!(writer.features_without_path, 0);
            assert!(writer.started_at.is_none());
            assert!(writer.step_started_at.is_none());
        }

        #[test]
        fn libtest_clone_preserves_state() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            writer.passed = 5;
            writer.failed = 2;
            writer.parsed_all = true;
            
            let cloned = writer.clone();
            
            assert_eq!(cloned.passed, 5);
            assert_eq!(cloned.failed, 2);
            assert!(cloned.parsed_all);
        }

        #[test]
        fn libtest_debug_implementation() {
            let writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let debug_str = format!("{writer:?}");
            assert!(debug_str.contains("Libtest"));
        }
    }

    mod writer_trait_tests {
        use super::*;

        #[test]
        fn libtest_has_correct_cli_type() {
            // Test that the Writer trait is implemented with correct Cli type
            fn assert_writer_cli<W: Writer<MockWorld>>() -> W::Cli {
                panic!("This function is only for type checking")
            }
            
            // This should compile without errors
            let _: fn() -> Cli = || assert_writer_cli::<Libtest<MockWorld, Vec<u8>>>();
        }
    }

    mod stats_trait_tests {
        use super::*;

        #[test]
        fn libtest_stats_trait_implementation() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            writer.passed = 10;
            writer.ignored = 5;
            writer.failed = 3;
            writer.retried = 2;
            writer.parsing_errors = 1;
            writer.hook_errors = 1;
            
            assert_eq!(writer.passed_steps(), 10);
            assert_eq!(writer.skipped_steps(), 5);
            assert_eq!(writer.failed_steps(), 3);
            assert_eq!(writer.retried_steps(), 2);
            assert_eq!(writer.parsing_errors(), 1);
            assert_eq!(writer.hook_errors(), 1);
        }
    }

    mod output_formatter_tests {
        use super::*;

        #[test]
        fn libtest_output_formatter_implementation() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            
            // Test output_mut access
            let output = writer.output_mut();
            output.extend_from_slice(b"test");
            
            assert_eq!(writer.output, b"test");
        }
    }

    mod constructor_tests {
        use super::*;

        #[test]
        fn libtest_new_returns_normalized() {
            // We can't easily test the actual normalization without complex setup,
            // but we can verify the function exists and has correct signature
            let _writer = Libtest::<MockWorld, Vec<u8>>::new(Vec::new());
            // If this compiles, the function exists with the right signature
        }

        #[test]
        fn libtest_stdout_constructor() {
            // Similar to above, just test that it compiles
            let _writer = Libtest::<MockWorld>::stdout();
        }
    }

    mod arbitrary_trait_tests {
        use super::*;

        #[tokio::test]
        async fn libtest_arbitrary_write() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            
            // Test writing a string value
            writer.write("test string").await;
            
            // The exact output format depends on WriteStrExt implementation,
            // but we can verify that something was written
            assert!(!writer.output.is_empty());
        }

        #[tokio::test]
        async fn libtest_arbitrary_write_string() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            
            // Test writing a String value
            writer.write(String::from("test string")).await;
            
            assert!(!writer.output.is_empty());
        }
    }

    mod type_aliases_tests {
        use super::*;

        #[test]
        fn or_type_alias_compilation() {
            // Test that the Or type alias compiles correctly
            fn _test_or_type() -> Or<MockWorld, writer::Basic> {
                panic!("Type checking only")
            }
        }

        #[test]
        fn or_basic_type_alias_compilation() {
            // Test that the OrBasic type alias compiles correctly
            fn _test_or_basic_type() -> OrBasic<MockWorld> {
                panic!("Type checking only") 
            }
        }
    }
}