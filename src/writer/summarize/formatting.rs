//! Summary formatting utilities for generating styled output.

use std::borrow::Cow;

use itertools::Itertools as _;

use crate::writer::out::Styles;

use super::{core::Summarize, stats::Stats};

/// Extension trait for [`Styles`] to provide summary formatting functionality.
///
/// This trait extends the base [`Styles`] implementation with methods specifically
/// designed for formatting test execution summaries, including feature counts,
/// scenario statistics, step statistics, and error reporting.
pub trait SummaryFormatter {
    /// Generates a formatted summary [`String`] from a [`Summarize`] writer.
    ///
    /// The summary includes:
    /// - Number of features and rules processed
    /// - Scenario statistics (passed, skipped, failed, retried)
    /// - Step statistics (passed, skipped, failed, retried)
    /// - Parsing and hook errors
    ///
    /// All sections are formatted with appropriate styling and colors.
    fn summary<W>(&self, summary: &Summarize<W>) -> String;

    /// Formats [`Stats`] for terminal output with colors and styling.
    ///
    /// This method takes statistics (passed, skipped, failed, retried counts)
    /// and formats them as a colored, comma-separated string suitable for
    /// display in the summary.
    ///
    /// Returns an empty string if all statistics are zero.
    fn format_stats(&self, stats: Stats) -> Cow<'static, str>;

    /// Adds plural suffix to a word based on the given count.
    ///
    /// If `num` is 1, returns the singular form. Otherwise, appends "s".
    /// The result is wrapped in bold styling.
    fn maybe_plural(
        &self,
        singular: impl Into<Cow<'static, str>>,
        num: usize,
    ) -> Cow<'static, str>;
}

impl SummaryFormatter for Styles {
    fn summary<W>(&self, summary: &Summarize<W>) -> String {
        let features = self.maybe_plural("feature", summary.features_count());

        let rules = if summary.rules_count() > 0 {
            format!("{}\n", self.maybe_plural("rule", summary.rules_count()))
        } else {
            String::new()
        };

        let scenarios =
            self.maybe_plural("scenario", summary.scenarios_stats().total());
        let scenarios_stats = self.format_stats(*summary.scenarios_stats());

        let steps = self.maybe_plural("step", summary.steps_stats().total());
        let steps_stats = self.format_stats(*summary.steps_stats());

        let parsing_errors = if summary.parsing_errors_count() > 0 {
            self.err(self.maybe_plural("parsing error", summary.parsing_errors_count()))
        } else {
            "".into()
        };

        let hook_errors = if summary.failed_hooks_count() > 0 {
            self.err(self.maybe_plural("hook error", summary.failed_hooks_count()))
        } else {
            "".into()
        };

        let comma = if !parsing_errors.is_empty() && !hook_errors.is_empty() {
            self.err(", ")
        } else {
            "".into()
        };

        format!(
            "{summary}\n{features}\n{rules}{scenarios}{scenarios_stats}\n\
             {steps}{steps_stats}\n{parsing_errors}{comma}{hook_errors}",
            summary = self.bold(self.header("[Summary]")),
        )
        .trim_end_matches('\n')
        .to_owned()
    }

    fn format_stats(&self, stats: Stats) -> Cow<'static, str> {
        let mut formatted = [
            if stats.passed > 0 {
                self.bold(self.ok(format!("{} passed", stats.passed)))
            } else {
                "".into()
            },
            if stats.skipped > 0 {
                self.bold(self.skipped(format!("{} skipped", stats.skipped)))
            } else {
                "".into()
            },
            if stats.failed > 0 {
                self.bold(self.err(format!("{} failed", stats.failed)))
            } else {
                "".into()
            },
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .join(&self.bold(", "));

        if stats.retried > 0 {
            formatted.push_str(" with ");
            formatted.push_str(&self.bold(self.retry(format!(
                "{} retr{}",
                stats.retried,
                if stats.retried == 1 { "y" } else { "ies" },
            ))));
        }

        if formatted.is_empty() {
            "".into()
        } else {
            self.bold(format!(
                " {}{formatted}{}",
                self.bold("("),
                self.bold(")"),
            ))
        }
    }

    fn maybe_plural(
        &self,
        singular: impl Into<Cow<'static, str>>,
        num: usize,
    ) -> Cow<'static, str> {
        self.bold(format!(
            "{num} {}{}",
            singular.into(),
            if num == 1 { "" } else { "s" },
        ))
    }
}

/// Utility functions for formatting summary components.
#[derive(Clone, Copy, Debug)]
pub struct SummaryUtils;

impl SummaryUtils {
    /// Formats a count with optional plural suffix.
    ///
    /// This is a utility function that doesn't apply styling, useful for
    /// testing or when styling is not desired.
    #[must_use]
    pub fn format_count(singular: &str, count: usize) -> String {
        format!(
            "{count} {singular}{}",
            if count == 1 { "" } else { "s" }
        )
    }

    /// Calculates the total error count from parsing and hook errors.
    #[must_use]
    pub const fn total_errors(parsing_errors: usize, hook_errors: usize) -> usize {
        parsing_errors + hook_errors
    }

    /// Determines if a summary has any failures (failed steps/scenarios or errors).
    #[must_use]
    pub fn has_any_failures(
        stats: &Stats,
        parsing_errors: usize,
        hook_errors: usize,
    ) -> bool {
        stats.has_failures() || parsing_errors > 0 || hook_errors > 0
    }

    /// Creates a simple text summary without styling.
    ///
    /// This is useful for logging or when output styling is not supported.
    #[must_use]
    pub fn plain_text_summary<W>(summary: &Summarize<W>) -> String {
        let mut parts = Vec::new();

        parts.push(Self::format_count("feature", summary.features_count()));

        if summary.rules_count() > 0 {
            parts.push(Self::format_count("rule", summary.rules_count()));
        }

        let scenarios = summary.scenarios_stats();
        parts.push(format!(
            "{} (passed: {}, skipped: {}, failed: {}, retried: {})",
            Self::format_count("scenario", scenarios.total()),
            scenarios.passed,
            scenarios.skipped,
            scenarios.failed,
            scenarios.retried,
        ));

        let steps = summary.steps_stats();
        parts.push(format!(
            "{} (passed: {}, skipped: {}, failed: {}, retried: {})",
            Self::format_count("step", steps.total()),
            steps.passed,
            steps.skipped,
            steps.failed,
            steps.retried,
        ));

        if summary.parsing_errors_count() > 0 {
            parts.push(Self::format_count("parsing error", summary.parsing_errors_count()));
        }

        if summary.failed_hooks_count() > 0 {
            parts.push(Self::format_count("hook error", summary.failed_hooks_count()));
        }

        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::Coloring;
    use crate::test_utils::common::{EmptyCli, TestWorld};
    use crate::{Writer, parser, Event};

    fn create_test_stats() -> Stats {
        Stats {
            passed: 5,
            skipped: 2,
            failed: 1,
            retried: 3,
        }
    }

    #[test]
    fn summary_utils_format_count_singular() {
        assert_eq!(SummaryUtils::format_count("test", 1), "1 test");
    }

    #[test]
    fn summary_utils_format_count_plural() {
        assert_eq!(SummaryUtils::format_count("test", 0), "0 tests");
        assert_eq!(SummaryUtils::format_count("test", 2), "2 tests");
        assert_eq!(SummaryUtils::format_count("test", 10), "10 tests");
    }

    #[test]
    fn summary_utils_total_errors() {
        assert_eq!(SummaryUtils::total_errors(5, 3), 8);
        assert_eq!(SummaryUtils::total_errors(0, 0), 0);
        assert_eq!(SummaryUtils::total_errors(10, 0), 10);
        assert_eq!(SummaryUtils::total_errors(0, 7), 7);
    }

    #[test]
    fn summary_utils_has_any_failures() {
        let stats_with_failures = Stats {
            passed: 5, skipped: 2, failed: 1, retried: 0
        };
        let stats_without_failures = Stats {
            passed: 5, skipped: 2, failed: 0, retried: 0
        };

        // Test with failed stats
        assert!(SummaryUtils::has_any_failures(&stats_with_failures, 0, 0));
        
        // Test with parsing errors
        assert!(SummaryUtils::has_any_failures(&stats_without_failures, 1, 0));
        
        // Test with hook errors
        assert!(SummaryUtils::has_any_failures(&stats_without_failures, 0, 1));
        
        // Test with no failures
        assert!(!SummaryUtils::has_any_failures(&stats_without_failures, 0, 0));
    }

    #[test]
    fn styles_maybe_plural_singular() {
        let styles = Styles::new();
        let result = styles.maybe_plural("test", 1);
        assert!(result.contains("1 test"));
        assert!(!result.contains("tests"));
    }

    #[test]
    fn styles_maybe_plural_plural() {
        let styles = Styles::new();
        
        let result_zero = styles.maybe_plural("test", 0);
        assert!(result_zero.contains("0 tests"));
        
        let result_many = styles.maybe_plural("test", 5);
        assert!(result_many.contains("5 tests"));
    }

    #[test]
    fn styles_format_stats_empty() {
        let styles = Styles::new();
        let stats = Stats::new();
        let result = styles.format_stats(stats);
        assert_eq!(result, "");
    }

    #[test]
    fn styles_format_stats_with_values() {
        let mut styles = Styles::new();
        styles.apply_coloring(Coloring::Never); // Disable coloring for predictable output
        
        let stats = Stats {
            passed: 5,
            skipped: 2,
            failed: 1,
            retried: 0,
        };
        
        let result = styles.format_stats(stats);
        assert!(result.contains("5 passed"));
        assert!(result.contains("2 skipped"));
        assert!(result.contains("1 failed"));
        assert!(!result.contains("retries"));
    }

    #[test]
    fn styles_format_stats_with_retries() {
        let mut styles = Styles::new();
        styles.apply_coloring(Coloring::Never);
        
        let stats = Stats {
            passed: 3,
            skipped: 0,
            failed: 0,
            retried: 1,
        };
        
        let result = styles.format_stats(stats);
        assert!(result.contains("3 passed"));
        assert!(result.contains("1 retry"));
    }

    #[test]
    fn styles_format_stats_multiple_retries() {
        let mut styles = Styles::new();
        styles.apply_coloring(Coloring::Never);
        
        let stats = Stats {
            passed: 3,
            skipped: 0,
            failed: 0,
            retried: 5,
        };
        
        let result = styles.format_stats(stats);
        assert!(result.contains("3 passed"));
        assert!(result.contains("5 retries"));
    }

    #[derive(Debug, Clone)]
    struct MockWriter;

    impl<W: crate::World> Writer<W> for MockWriter {
        type Cli = EmptyCli;

        async fn handle_event(
            &mut self,
            _event: parser::Result<Event<crate::event::Cucumber<W>>>,
            _cli: &Self::Cli,
        ) {
            // No-op for testing
        }
    }

    impl<W: crate::World> crate::writer::Arbitrary<W, String> for MockWriter {
        async fn write(&mut self, _val: String) {}
    }

    impl crate::writer::NonTransforming for MockWriter {}

    #[test]
    fn summary_utils_plain_text_summary() {
        use super::super::core::Summarize;
        
        let writer = MockWriter;
        let mut summary = Summarize::new(writer);
        
        // Set some test values
        summary.features = 2;
        summary.rules = 1;
        summary.scenarios = Stats { passed: 8, skipped: 2, failed: 1, retried: 3 };
        summary.steps = Stats { passed: 15, skipped: 3, failed: 2, retried: 5 };
        summary.parsing_errors = 1;
        summary.failed_hooks = 0;
        
        let result = SummaryUtils::plain_text_summary(&summary);
        
        assert!(result.contains("2 features"));
        assert!(result.contains("1 rule"));
        assert!(result.contains("11 scenarios")); // 8+2+1
        assert!(result.contains("passed: 8"));
        assert!(result.contains("skipped: 2"));
        assert!(result.contains("failed: 1"));
        assert!(result.contains("retried: 3"));
        assert!(result.contains("20 steps")); // 15+3+2
        assert!(result.contains("1 parsing error"));
        assert!(!result.contains("hook error"));
    }

    #[test]
    fn summary_utils_plain_text_summary_no_rules() {
        use super::super::core::Summarize;
        
        let writer = MockWriter;
        let summary = Summarize::new(writer);
        
        let result = SummaryUtils::plain_text_summary(&summary);
        
        assert!(result.contains("0 features"));
        assert!(!result.contains("rule"));
        assert!(result.contains("0 scenarios"));
        assert!(result.contains("0 steps"));
        assert!(!result.contains("parsing error"));
        assert!(!result.contains("hook error"));
    }
}