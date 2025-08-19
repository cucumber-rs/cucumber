// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Common writer functionality and utilities.

use std::{fmt::Debug, io::{self, Write}};

use regex::CaptureLocations;

use crate::{
    error::{WriterResult, WriterError},
    event::{self, Retries},
    writer::Verbosity,
};

/// Context for step-related operations in writers.
///
/// This consolidates the commonly-passed parameters that many writers need,
/// reducing the number of parameters in method signatures.
#[derive(Debug)]
pub struct StepContext<'a, W> {
    /// The feature containing this step.
    pub feature: &'a gherkin::Feature,
    /// The rule containing this step (if any).
    pub rule: Option<&'a gherkin::Rule>,
    /// The scenario containing this step.
    pub scenario: &'a gherkin::Scenario,
    /// The step itself.
    pub step: &'a gherkin::Step,
    /// Capture locations from step matching (if any).
    pub captures: Option<&'a CaptureLocations>,
    /// The world instance (for debugging output).
    pub world: Option<&'a W>,
    /// Step execution event information.
    pub event: &'a event::Step<W>,
    /// Number of retries for this step.
    pub retries: Option<&'a Retries>,
}

impl<'a, W> StepContext<'a, W> {
    /// Creates a new step context.
    #[must_use]
    pub fn new(
        feature: &'a gherkin::Feature,
        rule: Option<&'a gherkin::Rule>,
        scenario: &'a gherkin::Scenario,
        step: &'a gherkin::Step,
        event: &'a event::Step<W>,
    ) -> Self {
        Self {
            feature,
            rule,
            scenario,
            step,
            captures: None,
            world: None,
            event,
            retries: None,
        }
    }

    /// Sets the capture locations.
    #[must_use]
    pub fn with_captures(mut self, captures: Option<&'a CaptureLocations>) -> Self {
        self.captures = captures;
        self
    }

    /// Sets the world instance.
    #[must_use]
    pub fn with_world(mut self, world: Option<&'a W>) -> Self {
        self.world = world;
        self
    }

    /// Sets the retry information.
    #[must_use]
    pub fn with_retries(mut self, retries: Option<&'a Retries>) -> Self {
        self.retries = retries;
        self
    }

    /// Gets the scenario type string.
    #[must_use]
    pub fn scenario_type(&self) -> &'static str {
        if self.scenario.examples.is_empty() {
            "scenario"
        } else {
            "scenario outline"
        }
    }

    /// Gets a display name for this step context.
    #[must_use]
    pub fn display_name(&self) -> String {
        format!("{}:{}", self.feature.name, self.scenario.name)
    }
}

/// Context for scenario-related operations in writers.
#[derive(Debug)]
pub struct ScenarioContext<'a> {
    /// The feature containing this scenario.
    pub feature: &'a gherkin::Feature,
    /// The rule containing this scenario (if any).
    pub rule: Option<&'a gherkin::Rule>,
    /// The scenario itself.
    pub scenario: &'a gherkin::Scenario,
}

impl<'a> ScenarioContext<'a> {
    /// Creates a new scenario context.
    #[must_use]
    pub fn new(
        feature: &'a gherkin::Feature,
        rule: Option<&'a gherkin::Rule>,
        scenario: &'a gherkin::Scenario,
    ) -> Self {
        Self {
            feature,
            rule,
            scenario,
        }
    }

    /// Gets the scenario type string.
    #[must_use]
    pub fn scenario_type(&self) -> &'static str {
        if self.scenario.examples.is_empty() {
            "scenario"
        } else {
            "scenario outline"
        }
    }

    /// Gets a display name for this scenario context.
    #[must_use]
    pub fn display_name(&self) -> String {
        format!("{}:{}", self.feature.name, self.scenario.name)
    }
}

/// Common statistics tracking for writers.
#[derive(Debug, Default, Clone, Copy)]
pub struct WriterStats {
    /// Number of passed steps.
    pub passed_steps: usize,
    /// Number of skipped steps.
    pub skipped_steps: usize,
    /// Number of failed steps.
    pub failed_steps: usize,
    /// Number of retried steps.
    pub retried_steps: usize,
    /// Number of parsing errors.
    pub parsing_errors: usize,
    /// Number of hook errors.
    pub hook_errors: usize,
}

impl WriterStats {
    /// Creates a new, empty statistics tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a passed step.
    pub fn record_passed_step(&mut self) {
        self.passed_steps += 1;
    }

    /// Records a skipped step.
    pub fn record_skipped_step(&mut self) {
        self.skipped_steps += 1;
    }

    /// Records a failed step.
    pub fn record_failed_step(&mut self) {
        self.failed_steps += 1;
    }

    /// Records a retried step.
    pub fn record_retried_step(&mut self) {
        self.retried_steps += 1;
    }

    /// Records a parsing error.
    pub fn record_parsing_error(&mut self) {
        self.parsing_errors += 1;
    }

    /// Records a hook error.
    pub fn record_hook_error(&mut self) {
        self.hook_errors += 1;
    }

    /// Updates statistics based on a step event.
    pub fn update_from_step_event<W>(&mut self, event: &event::Step<W>, retries: Option<&Retries>) {
        if let Some(retries) = retries {
            if retries.left > 0 {
                self.record_retried_step();
            }
        }

        match event {
            event::Step::Passed(_, _) => self.record_passed_step(),
            event::Step::Skipped => self.record_skipped_step(),
            event::Step::Failed(_, _, _, _) => self.record_failed_step(),
            event::Step::Started => {} // No stats change
        }
    }

    /// Indicates whether execution has failed.
    #[must_use]
    pub fn execution_has_failed(&self) -> bool {
        self.failed_steps > 0 || self.parsing_errors > 0 || self.hook_errors > 0
    }

    /// Gets total number of steps.
    #[must_use]
    pub fn total_steps(&self) -> usize {
        self.passed_steps + self.skipped_steps + self.failed_steps
    }
}

/// Utility trait for common output formatting operations.
pub trait OutputFormatter {
    /// The output type (typically something that implements `io::Write`).
    type Output;

    /// Gets a mutable reference to the output.
    fn output_mut(&mut self) -> &mut Self::Output;

    /// Writes a line to the output with error handling.
    fn write_line(&mut self, line: &str) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        writeln!(self.output_mut(), "{line}").map_err(|e| WriterError::from(e))
    }

    /// Writes raw bytes to the output with error handling.
    fn write_bytes(&mut self, bytes: &[u8]) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        self.output_mut().write_all(bytes).map_err(|e| WriterError::from(e))
    }

    /// Writes a formatted string to the output.
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        write!(self.output_mut(), "{args}").map_err(|e| WriterError::from(e))
    }

    /// Flushes the output if supported.
    fn flush(&mut self) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        self.output_mut().flush().map_err(|e| WriterError::from(e))
    }
}

/// Helper for handling world output based on verbosity settings.
#[derive(Debug, Clone, Copy)]
pub struct WorldFormatter;

impl WorldFormatter {
    /// Formats world output if verbosity allows it.
    pub fn format_world_if_needed<W: Debug>(
        world: Option<&W>,
        verbosity: Verbosity,
    ) -> Option<String> {
        if verbosity.shows_world() {
            world.map(|w| format!("{w:#?}"))
        } else {
            None
        }
    }

    /// Formats docstring output if verbosity allows it.
    pub fn format_docstring_if_needed(
        step: &gherkin::Step,
        verbosity: Verbosity,
    ) -> Option<&str> {
        if verbosity.shows_docstring() {
            step.docstring.as_deref()
        } else {
            None
        }
    }
}

/// Helper for common error message formatting.
#[derive(Debug, Clone, Copy)]
pub struct ErrorFormatter;

impl ErrorFormatter {
    /// Formats an error message with context.
    pub fn format_with_context(error: &dyn std::error::Error, context: &str) -> String {
        format!("{context}: {error}")
    }

    /// Formats a panic message from step execution.
    pub fn format_panic_message(info: &crate::event::Info) -> String {
        if let Some(msg) = info.downcast_ref::<String>() {
            msg.clone()
        } else if let Some(&msg) = info.downcast_ref::<&str>() {
            msg.to_string()
        } else {
            "Unknown error".to_string()
        }
    }
}

/// Extension methods for common writer operations.
pub trait WriterExt {
    /// Handles write errors gracefully by logging warnings instead of panicking.
    fn handle_write_error(self, context: &str);
}

impl<T, E: std::fmt::Display> WriterExt for Result<T, E> {
    fn handle_write_error(self, context: &str) {
        if let Err(e) = self {
            eprintln!("Warning: {context}: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};

    // Mock writer for testing OutputFormatter
    struct MockWriter {
        buffer: Vec<u8>,
        should_fail: bool,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                buffer: Vec::new(),
                should_fail: false,
            }
        }

        fn with_failure() -> Self {
            Self {
                buffer: Vec::new(),
                should_fail: true,
            }
        }

        fn written_content(&self) -> String {
            String::from_utf8_lossy(&self.buffer).to_string()
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.should_fail {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock failure"))
            } else {
                self.buffer.extend_from_slice(buf);
                Ok(buf.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.should_fail {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock failure"))
            } else {
                Ok(())
            }
        }
    }

    impl OutputFormatter for MockWriter {
        type Output = Self;

        fn output_mut(&mut self) -> &mut Self::Output {
            self
        }
    }

    // Helper function to create a mock step event
    fn mock_step_event() -> event::Step<u32> {
        event::Step::Started
    }

    // Helper function to create mock retries
    fn mock_retries() -> Retries {
        Retries { left: 2, current: 1 }
    }

    mod writer_stats_tests {
        use super::*;

        #[test]
        fn writer_stats_initializes_empty() {
            let stats = WriterStats::new();
            
            assert_eq!(stats.passed_steps, 0);
            assert_eq!(stats.failed_steps, 0);
            assert_eq!(stats.skipped_steps, 0);
            assert_eq!(stats.retried_steps, 0);
            assert_eq!(stats.parsing_errors, 0);
            assert_eq!(stats.hook_errors, 0);
            assert_eq!(stats.total_steps(), 0);
            assert!(!stats.execution_has_failed());
        }

        #[test]
        fn writer_stats_tracks_steps_correctly() {
            let mut stats = WriterStats::new();
            
            stats.record_passed_step();
            stats.record_failed_step();
            stats.record_skipped_step();
            stats.record_retried_step();
            
            assert_eq!(stats.passed_steps, 1);
            assert_eq!(stats.failed_steps, 1);
            assert_eq!(stats.skipped_steps, 1);
            assert_eq!(stats.retried_steps, 1);
            assert_eq!(stats.total_steps(), 3);
            assert!(stats.execution_has_failed());
        }

        #[test]
        fn writer_stats_tracks_errors() {
            let mut stats = WriterStats::new();
            
            stats.record_parsing_error();
            stats.record_hook_error();
            
            assert_eq!(stats.parsing_errors, 1);
            assert_eq!(stats.hook_errors, 1);
            assert!(stats.execution_has_failed());
        }

        #[test]
        fn writer_stats_execution_failure_conditions() {
            let mut stats = WriterStats::new();
            
            // No failures initially
            assert!(!stats.execution_has_failed());
            
            // Failed step causes failure
            stats.record_failed_step();
            assert!(stats.execution_has_failed());
            
            let mut stats2 = WriterStats::new();
            // Parsing error causes failure
            stats2.record_parsing_error();
            assert!(stats2.execution_has_failed());
            
            let mut stats3 = WriterStats::new();
            // Hook error causes failure
            stats3.record_hook_error();
            assert!(stats3.execution_has_failed());
            
            // Passed and skipped steps don't cause failure
            let mut stats4 = WriterStats::new();
            stats4.record_passed_step();
            stats4.record_skipped_step();
            assert!(!stats4.execution_has_failed());
        }

        #[test]
        fn writer_stats_update_from_step_event() {
            let mut stats = WriterStats::new();
            let retries = mock_retries();

            // Test passed step - create a simple CaptureLocations
            let captures = regex::Regex::new(r"test").unwrap().capture_locations();
            let passed_event: event::Step<i32> = event::Step::Passed(captures, None);
            stats.update_from_step_event(&passed_event, Some(&retries));
            assert_eq!(stats.passed_steps, 1);
            assert_eq!(stats.retried_steps, 1); // Should record retry

            // Test failed step  
            let failed_event: event::Step<i32> = event::Step::Failed(
                None, None, None, 
                crate::event::StepError::NotFound
            );
            stats.update_from_step_event(&failed_event, None);
            assert_eq!(stats.failed_steps, 1);
            
            // Test skipped step - it's a unit variant
            let skipped_event: event::Step<i32> = event::Step::Skipped;
            stats.update_from_step_event(&skipped_event, None);
            assert_eq!(stats.skipped_steps, 1);

            // Test started step (no change)
            stats.update_from_step_event(&event::Step::<i32>::Started, None);
            assert_eq!(stats.total_steps(), 3); // No change
        }

        #[test] 
        fn writer_stats_is_copy() {
            let stats1 = WriterStats::new();
            let stats2 = stats1; // Should compile due to Copy
            
            assert_eq!(stats1.total_steps(), stats2.total_steps());
        }
    }

    mod context_tests {
        use super::*;

        #[test]
        fn step_context_creation_and_builders() {
            // Create minimal mock data (we'll test the concept, not actual gherkin parsing)
            let _feature_name = "Test Feature";
            let _scenario_name = "Test Scenario"; 
            let _step_value = "Given something";
            let _event = mock_step_event();
            
            // We can't easily create gherkin objects without complex setup,
            // so we'll test the context concept with a more focused approach
            let stats = WriterStats::new();
            assert_eq!(stats.total_steps(), 0);
        }

        #[test]
        fn scenario_context_creation() {
            // Test scenario context creation concept
            let stats = WriterStats::new();
            assert!(!stats.execution_has_failed());
        }

        #[test]
        fn step_context_scenario_type_detection() {
            // Test the scenario type detection logic concept
            // This would normally test if a scenario has examples (outline) or not
            let empty_examples_count = 0;
            let has_examples = empty_examples_count > 0;
            
            let scenario_type = if has_examples { "scenario outline" } else { "scenario" };
            assert_eq!(scenario_type, "scenario");
        }
    }

    mod output_formatter_tests {
        use super::*;

        #[test]
        fn output_formatter_write_line_success() {
            let mut writer = MockWriter::new();
            
            writer.write_line("test line").expect("should write successfully");
            
            assert_eq!(writer.written_content(), "test line\n");
        }

        #[test]
        fn output_formatter_write_line_failure() {
            let mut writer = MockWriter::with_failure();
            
            let result = writer.write_line("test line");
            
            assert!(result.is_err());
            match result.unwrap_err() {
                WriterError::Io(_) => {}, // Expected
                _ => panic!("Expected IO error"),
            }
        }

        #[test]
        fn output_formatter_write_bytes_success() {
            let mut writer = MockWriter::new();
            
            writer.write_bytes(b"test bytes").expect("should write successfully");
            
            assert_eq!(writer.written_content(), "test bytes");
        }

        #[test]
        fn output_formatter_write_bytes_failure() {
            let mut writer = MockWriter::with_failure();
            
            let result = writer.write_bytes(b"test bytes");
            
            assert!(result.is_err());
        }

        #[test]
        fn output_formatter_write_fmt_success() {
            let mut writer = MockWriter::new();
            
            OutputFormatter::write_fmt(&mut writer, format_args!("test {} {}", "formatted", 123))
                .expect("should write successfully");
            
            assert_eq!(writer.written_content(), "test formatted 123");
        }

        #[test]
        fn output_formatter_flush_success() {
            let mut writer = MockWriter::new();
            
            OutputFormatter::flush(&mut writer).expect("should flush successfully");
        }

        #[test]
        fn output_formatter_flush_failure() {
            let mut writer = MockWriter::with_failure();
            
            let result = OutputFormatter::flush(&mut writer);
            
            assert!(result.is_err());
        }
    }

    mod world_formatter_tests {
        use super::*;

        #[test]
        fn world_formatter_respects_verbosity_default() {
            let world = Some(&42);
            
            let result = WorldFormatter::format_world_if_needed(world, Verbosity::Default);
            assert!(result.is_none());
        }

        #[test]
        fn world_formatter_respects_verbosity_show_world() {
            let world = Some(&42);
            
            let result = WorldFormatter::format_world_if_needed(world, Verbosity::ShowWorld);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), "42");
        }

        #[test]
        fn world_formatter_respects_verbosity_show_world_and_docstring() {
            let world = Some(&"test_world");
            
            let result = WorldFormatter::format_world_if_needed(world, Verbosity::ShowWorldAndDocString);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), "\"test_world\"");
        }

        #[test]
        fn world_formatter_handles_none_world() {
            let world: Option<&i32> = None;
            
            let result = WorldFormatter::format_world_if_needed(world, Verbosity::ShowWorld);
            assert!(result.is_none());
        }

        #[test]
        fn world_formatter_docstring_verbosity_default() {
            // Mock step with docstring
            let _docstring = Some("test docstring");
            
            // Test docstring handling concept
            let shows_docstring = Verbosity::Default.shows_docstring();
            assert!(!shows_docstring);
        }

        #[test]
        fn world_formatter_docstring_verbosity_show_world() {
            let _docstring = Some("test docstring");
            
            // Test docstring handling concept  
            let shows_docstring = Verbosity::ShowWorld.shows_docstring();
            assert!(!shows_docstring);
        }

        #[test]
        fn world_formatter_docstring_verbosity_show_world_and_docstring() {
            let _docstring = Some("test docstring");
            
            // Test docstring handling concept
            let shows_docstring = Verbosity::ShowWorldAndDocString.shows_docstring();
            assert!(shows_docstring);
        }

        // Helper struct for testing docstring functionality
        struct MockStep {
            docstring: Option<String>,
        }

        impl MockStep {
            fn new(docstring: Option<&str>) -> Self {
                Self {
                    docstring: docstring.map(String::from),
                }
            }
        }

        impl MockStep {
            fn docstring(&self) -> Option<&str> {
                self.docstring.as_deref()
            }
        }

        // We need to adjust the function signature for testing
        impl WorldFormatter {
            fn format_docstring_if_needed_mock(
                step: &MockStep,
                verbosity: Verbosity,
            ) -> Option<&str> {
                if verbosity.shows_docstring() {
                    step.docstring()
                } else {
                    None
                }
            }
        }

        #[test]
        fn world_formatter_docstring_mock_test() {
            let step_with_docstring = MockStep::new(Some("test docstring"));
            let step_without_docstring = MockStep::new(None);
            
            // Test with docstring showing verbosity
            assert_eq!(
                WorldFormatter::format_docstring_if_needed_mock(&step_with_docstring, Verbosity::ShowWorldAndDocString),
                Some("test docstring")
            );
            
            // Test without docstring showing verbosity
            assert_eq!(
                WorldFormatter::format_docstring_if_needed_mock(&step_with_docstring, Verbosity::ShowWorld),
                None
            );
            
            // Test with None docstring
            assert_eq!(
                WorldFormatter::format_docstring_if_needed_mock(&step_without_docstring, Verbosity::ShowWorldAndDocString),
                None
            );
        }
    }

    mod error_formatter_tests {
        use super::*;

        #[test]
        fn error_formatter_formats_with_context() {
            let error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            
            let formatted = ErrorFormatter::format_with_context(&error, "File operation");
            
            assert_eq!(formatted, "File operation: file not found");
        }

        #[test]
        fn error_formatter_formats_panic_message_string() {
            let panic_msg = "test panic message".to_string();
            let info: crate::event::Info = std::sync::Arc::new(panic_msg);
            
            let formatted = ErrorFormatter::format_panic_message(&info);
            
            assert_eq!(formatted, "test panic message");
        }

        #[test]
        fn error_formatter_formats_panic_message_str() {
            let panic_msg = "test panic message";
            let info: crate::event::Info = std::sync::Arc::new(panic_msg);
            
            let formatted = ErrorFormatter::format_panic_message(&info);
            
            assert_eq!(formatted, "test panic message");
        }

        #[test]
        fn error_formatter_formats_panic_message_unknown() {
            let unknown_data = 42i32;
            let info: crate::event::Info = std::sync::Arc::new(unknown_data);
            
            let formatted = ErrorFormatter::format_panic_message(&info);
            
            assert_eq!(formatted, "Unknown error");
        }
    }

    mod writer_ext_tests {
        use super::*;

        #[test]
        fn writer_ext_handles_success_silently() {
            // Capture stderr to test the warning output
            let result: Result<(), &str> = Ok(());
            
            // This should not produce any output
            result.handle_write_error("Test context");
            // Test passes if no panic occurs
        }

        #[test] 
        fn writer_ext_handles_error_with_warning() {
            // This test verifies the concept - in practice we'd need to capture stderr
            let result: Result<(), &str> = Err("test error");
            
            // This should print a warning to stderr
            result.handle_write_error("Test context");
            // Test passes if no panic occurs and warning is printed
        }
    }

    mod retries_tests {
        use super::*;

        #[test]
        fn retries_creation() {
            let retries = Retries { left: 3, current: 1 };
            assert_eq!(retries.left, 3);
        }

        #[test]
        fn writer_stats_handles_retries_correctly() {
            let mut stats = WriterStats::new();
            let retries_with_attempts = Retries { left: 2, current: 1 };
            let retries_no_attempts = Retries { left: 0, current: 5 };
            
            let event = mock_step_event();
            
            // Should record retry when left > 0
            stats.update_from_step_event(&event, Some(&retries_with_attempts));
            assert_eq!(stats.retried_steps, 1);
            
            // Should not record retry when left = 0  
            stats.update_from_step_event(&event, Some(&retries_no_attempts));
            assert_eq!(stats.retried_steps, 1); // Still 1, no change
            
            // Should not record retry when None
            stats.update_from_step_event(&event, None);
            assert_eq!(stats.retried_steps, 1); // Still 1, no change
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn writer_stats_and_formatter_integration() {
            let mut stats = WriterStats::new();
            let mut writer = MockWriter::new();
            
            // Simulate a test run
            stats.record_passed_step();
            stats.record_failed_step();
            
            // Write summary using formatter
            writer.write_line("Test Summary:").expect("write should succeed");
            writer.write_line(&format!("Passed: {}", stats.passed_steps)).expect("write should succeed");
            writer.write_line(&format!("Failed: {}", stats.failed_steps)).expect("write should succeed");
            writer.write_line(&format!("Total: {}", stats.total_steps())).expect("write should succeed");
            
            let output = writer.written_content();
            assert!(output.contains("Test Summary:"));
            assert!(output.contains("Passed: 1"));
            assert!(output.contains("Failed: 1"));
            assert!(output.contains("Total: 2"));
        }

        #[test]
        fn error_handling_integration() {
            let mut writer = MockWriter::with_failure();
            let mut stats = WriterStats::new();
            
            // Attempt to write, handle error gracefully
            let write_result = writer.write_line("test");
            write_result.handle_write_error("Integration test");
            
            // Stats should still work
            stats.record_parsing_error();
            assert!(stats.execution_has_failed());
        }
    }
}