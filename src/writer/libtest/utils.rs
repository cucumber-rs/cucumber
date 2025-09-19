// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Utility functions for the libtest writer.

use std::{fmt::Debug, io, time::{Duration, SystemTime}};

use either::Either;
use itertools::Itertools as _;

use crate::{
    World,
    event::{self, Metadata, Retries},
    writer::basic::trim_path,
};

use super::{cli::Cli, writer::Libtest};

/// Indicator, whether a [`Step`] is [`Background`] or not.
///
/// [`Background`]: event::Scenario::Background
/// [`Step`]: gherkin::Step
pub type IsBackground = bool;

/// Utility functions for libtest writer operations.
pub struct LibtestUtils;

impl LibtestUtils {
    /// Generates test case name.
    pub fn test_case_name<W, Out: io::Write>(
        writer: &mut Libtest<W, Out>,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        step: Either<event::HookType, (&gherkin::Step, IsBackground)>,
        retries: Option<Retries>,
    ) -> String {
        let feature_name = format!(
            "{}: {} {}",
            feature.keyword,
            feature.name,
            feature
                .path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .map_or_else(
                    || {
                        writer.features_without_path += 1;
                        writer.features_without_path.to_string()
                    },
                    |s| s.escape_default().to_string()
                ),
        );
        let rule_name = rule
            .as_ref()
            .map(|r| format!("{}: {}: {}", r.position.line, r.keyword, r.name));
        let scenario_name = format!(
            "{}: {}: {}{}",
            scenario.position.line,
            scenario.keyword,
            scenario.name,
            retries
                .filter(|r| r.current > 0)
                .map(|r| format!(
                    " | Retry attempt {}/{}",
                    r.current,
                    r.current + r.left,
                ))
                .unwrap_or_default(),
        );
        let step_name = match step {
            Either::Left(hook) => format!("{hook} hook"),
            Either::Right((step, is_bg)) => format!(
                "{}: {} {}{}",
                step.position.line,
                if is_bg {
                    feature
                        .background
                        .as_ref()
                        .map_or("Background", |bg| bg.keyword.as_str())
                } else {
                    ""
                },
                step.keyword,
                step.value,
            ),
        };

        [Some(feature_name), rule_name, Some(scenario_name), Some(step_name)]
            .into_iter()
            .flatten()
            .join("::")
    }

    /// Saves [`Step`] starting [`SystemTime`].
    ///
    /// [`Step`]: gherkin::Step
    pub fn step_started_at<W, Out: io::Write>(
        writer: &mut Libtest<W, Out>,
        meta: Metadata,
        cli: &Cli,
    ) {
        writer.step_started_at =
            Some(meta.at).filter(|_| cli.report_time.is_some());
    }

    /// Retrieves [`Duration`] since the last [`LibtestUtils::step_started_at()`]
    /// call.
    pub fn step_exec_time<W, Out: io::Write>(
        writer: &mut Libtest<W, Out>,
        meta: Metadata,
        cli: &Cli,
    ) -> Option<Duration> {
        let started = writer.step_started_at.take()?;
        meta.at
            .duration_since(started)
            .ok()
            .filter(|_| cli.report_time.is_some())
    }

    /// Formats a feature path for display, trimming common paths.
    pub fn format_feature_path(feature: &gherkin::Feature) -> String {
        feature
            .path
            .as_ref()
            .and_then(|p| p.to_str().map(trim_path))
            .unwrap_or(&feature.name)
            .to_string()
    }

    /// Formats step location information for output.
    pub fn format_step_location(
        feature: &gherkin::Feature,
        step: &gherkin::Step,
        location: Option<&crate::step::Location>,
    ) -> String {
        let base = format!(
            "{}:{}:{} (defined)",
            Self::format_feature_path(feature),
            step.position.line,
            step.position.col,
        );

        if let Some(loc) = location {
            format!(
                "{base}\n{}:{}:{} (matched)",
                loc.path, loc.line, loc.column,
            )
        } else {
            base
        }
    }

    /// Formats retry information for test names.
    pub fn format_retry_info(retries: Option<Retries>) -> String {
        retries
            .filter(|r| r.current > 0)
            .map(|r| format!(
                " | Retry attempt {}/{}",
                r.current,
                r.current + r.left,
            ))
            .unwrap_or_default()
    }

    /// Formats hook type for display.
    pub fn format_hook_type(hook: event::HookType) -> String {
        format!("{hook} hook")
    }

    /// Checks if a step should be considered a retry.
    pub fn is_retry_step(retries: Option<Retries>, error: &event::StepError) -> bool {
        retries.is_some_and(|r| {
            r.left > 0 && !matches!(error, event::StepError::NotFound)
        })
    }
}

/// Helper functions for working with test timing.
pub struct TimingUtils;

impl TimingUtils {
    /// Calculates execution time from start time to current time.
    pub fn calculate_exec_time(
        started_at: Option<SystemTime>,
        current_time: SystemTime,
    ) -> Option<Duration> {
        started_at
            .and_then(|started| current_time.duration_since(started).ok())
    }

    /// Converts duration to seconds as f64 for JSON serialization.
    pub fn duration_to_seconds(duration: Duration) -> f64 {
        duration.as_secs_f64()
    }

    /// Checks if timing should be reported based on CLI settings.
    pub fn should_report_time(cli: &Cli) -> bool {
        cli.report_time.is_some()
    }
}

/// Helper functions for working with background steps.
pub struct BackgroundUtils;

impl BackgroundUtils {
    /// Determines the background keyword to use for a step.
    pub fn get_background_keyword(feature: &gherkin::Feature) -> &str {
        feature
            .background
            .as_ref()
            .map_or("Background", |bg| bg.keyword.as_str())
    }

    /// Formats a background step name.
    pub fn format_background_step_name(
        feature: &gherkin::Feature,
        step: &gherkin::Step,
    ) -> String {
        format!(
            "{}: {} {}{}",
            step.position.line,
            Self::get_background_keyword(feature),
            step.keyword,
            step.value,
        )
    }

    /// Formats a regular step name.
    pub fn format_regular_step_name(step: &gherkin::Step) -> String {
        format!(
            "{}: {} {}",
            step.position.line,
            step.keyword,
            step.value,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[derive(Debug)]
    struct MockWorld;
    impl World for MockWorld {}

    // Helper function to create a mock feature
    fn mock_feature(name: &str, path: Option<&str>) -> gherkin::Feature {
        gherkin::Feature {
            keyword: "Feature".to_string(),
            name: name.to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::Position::new(1, 1),
            path: path.map(|p| PathBuf::from(p)),
        }
    }

    // Helper function to create a mock scenario
    fn mock_scenario(name: &str, line: u32) -> gherkin::Scenario {
        gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: name.to_string(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::Position::new(line, 1),
        }
    }

    // Helper function to create a mock step
    fn mock_step(keyword: &str, value: &str, line: u32) -> gherkin::Step {
        gherkin::Step {
            keyword: keyword.to_string(),
            value: value.to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::Position::new(line, 1),
        }
    }

    mod libtest_utils_tests {
        use super::*;

        #[test]
        fn test_case_name_generation_simple() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let feature = mock_feature("Test Feature", Some("test.feature"));
            let scenario = mock_scenario("Test Scenario", 5);
            let step = mock_step("Given", "some condition", 7);

            let name = LibtestUtils::test_case_name(
                &mut writer,
                &feature,
                None,
                &scenario,
                Either::Right((&step, false)),
                None,
            );

            assert!(name.contains("Feature: Test Feature"));
            assert!(name.contains("5: Scenario: Test Scenario"));
            assert!(name.contains("7: Given some condition"));
        }

        #[test]
        fn test_case_name_with_retries() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let feature = mock_feature("Test Feature", None);
            let scenario = mock_scenario("Test Scenario", 5);
            let step = mock_step("When", "something happens", 8);
            let retries = Retries { current: 2, left: 1 };

            let name = LibtestUtils::test_case_name(
                &mut writer,
                &feature,
                None,
                &scenario,
                Either::Right((&step, false)),
                Some(retries),
            );

            assert!(name.contains("Retry attempt 2/3"));
        }

        #[test]
        fn test_case_name_with_hook() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let feature = mock_feature("Test Feature", Some("test.feature"));
            let scenario = mock_scenario("Test Scenario", 5);

            let name = LibtestUtils::test_case_name(
                &mut writer,
                &feature,
                None,
                &scenario,
                Either::Left(event::HookType::Before),
                None,
            );

            assert!(name.contains("Before hook"));
            assert!(name.contains("Feature: Test Feature"));
            assert!(name.contains("5: Scenario: Test Scenario"));
        }

        #[test]
        fn test_case_name_background_step() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let mut feature = mock_feature("Test Feature", Some("test.feature"));
            feature.background = Some(gherkin::Background {
                keyword: "Background".to_string(),
                description: None,
                steps: vec![],
                span: gherkin::Span { start: 0, end: 0 },
                position: gherkin::Position::new(3, 1),
            });
            let scenario = mock_scenario("Test Scenario", 5);
            let step = mock_step("Given", "background condition", 4);

            let name = LibtestUtils::test_case_name(
                &mut writer,
                &feature,
                None,
                &scenario,
                Either::Right((&step, true)),
                None,
            );

            assert!(name.contains("4: Background Given background condition"));
        }

        #[test]
        fn test_case_name_increments_features_without_path() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let feature = mock_feature("Test Feature", None);
            let scenario = mock_scenario("Test Scenario", 5);
            let step = mock_step("Given", "condition", 7);

            // First call should increment to 1
            let name1 = LibtestUtils::test_case_name(
                &mut writer,
                &feature,
                None,
                &scenario,
                Either::Right((&step, false)),
                None,
            );
            assert!(name1.contains("Test Feature 1"));
            assert_eq!(writer.features_without_path, 1);

            // Second call should increment to 2
            let name2 = LibtestUtils::test_case_name(
                &mut writer,
                &feature,
                None,
                &scenario,
                Either::Right((&step, false)),
                None,
            );
            assert!(name2.contains("Test Feature 2"));
            assert_eq!(writer.features_without_path, 2);
        }

        #[test]
        fn step_started_at_with_timing() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli {
                report_time: Some(super::super::cli::ReportTime::Plain),
                ..Default::default()
            };
            let time = SystemTime::now();
            let meta = Metadata::new(time);

            LibtestUtils::step_started_at(&mut writer, meta, &cli);

            assert_eq!(writer.step_started_at, Some(time));
        }

        #[test]
        fn step_started_at_without_timing() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli {
                report_time: None,
                ..Default::default()
            };
            let meta = Metadata::new(SystemTime::now());

            LibtestUtils::step_started_at(&mut writer, meta, &cli);

            assert!(writer.step_started_at.is_none());
        }

        #[test]
        fn step_exec_time_calculation() {
            let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
            let cli = Cli {
                report_time: Some(super::super::cli::ReportTime::Plain),
                ..Default::default()
            };
            
            let start_time = SystemTime::now();
            writer.step_started_at = Some(start_time);
            
            let end_time = start_time + Duration::from_millis(500);
            let meta = Metadata::new(end_time);

            let exec_time = LibtestUtils::step_exec_time(&mut writer, meta, &cli);

            assert!(exec_time.is_some());
            let duration = exec_time.unwrap();
            assert!(duration >= Duration::from_millis(499)); // Allow small timing variance
            assert!(duration <= Duration::from_millis(501));
            assert!(writer.step_started_at.is_none()); // Should be consumed
        }

        #[test]
        fn format_feature_path_with_path() {
            let feature = mock_feature("Test Feature", Some("/long/path/to/test.feature"));
            let formatted = LibtestUtils::format_feature_path(&feature);
            
            // Should use trimmed path
            assert_eq!(formatted, "test.feature");
        }

        #[test]
        fn format_feature_path_without_path() {
            let feature = mock_feature("Test Feature", None);
            let formatted = LibtestUtils::format_feature_path(&feature);
            
            // Should fall back to feature name
            assert_eq!(formatted, "Test Feature");
        }

        #[test]
        fn format_retry_info_with_retries() {
            let retries = Retries { current: 1, left: 2 };
            let formatted = LibtestUtils::format_retry_info(Some(retries));
            
            assert_eq!(formatted, " | Retry attempt 1/3");
        }

        #[test]
        fn format_retry_info_first_attempt() {
            let retries = Retries { current: 0, left: 2 };
            let formatted = LibtestUtils::format_retry_info(Some(retries));
            
            // Should be empty for first attempt (current = 0)
            assert_eq!(formatted, "");
        }

        #[test]
        fn format_retry_info_none() {
            let formatted = LibtestUtils::format_retry_info(None);
            assert_eq!(formatted, "");
        }

        #[test]
        fn is_retry_step_conditions() {
            let retries_with_left = Retries { current: 1, left: 1 };
            let retries_no_left = Retries { current: 1, left: 0 };
            
            // Should retry with retries left and non-NotFound error
            assert!(LibtestUtils::is_retry_step(
                Some(retries_with_left), 
                &event::StepError::Panic(std::sync::Arc::new("test"))
            ));
            
            // Should not retry with no retries left
            assert!(!LibtestUtils::is_retry_step(
                Some(retries_no_left), 
                &event::StepError::Panic(std::sync::Arc::new("test"))
            ));
            
            // Should not retry NotFound error even with retries
            assert!(!LibtestUtils::is_retry_step(
                Some(retries_with_left), 
                &event::StepError::NotFound
            ));
            
            // Should not retry with no retries
            assert!(!LibtestUtils::is_retry_step(
                None, 
                &event::StepError::Panic(std::sync::Arc::new("test"))
            ));
        }
    }

    mod timing_utils_tests {
        use super::*;

        #[test]
        fn calculate_exec_time_success() {
            let start = SystemTime::now();
            let end = start + Duration::from_millis(100);
            
            let exec_time = TimingUtils::calculate_exec_time(Some(start), end);
            
            assert!(exec_time.is_some());
            let duration = exec_time.unwrap();
            assert!(duration >= Duration::from_millis(99)); // Allow small variance
            assert!(duration <= Duration::from_millis(101));
        }

        #[test]
        fn calculate_exec_time_no_start() {
            let end = SystemTime::now();
            let exec_time = TimingUtils::calculate_exec_time(None, end);
            assert!(exec_time.is_none());
        }

        #[test]
        fn duration_to_seconds_conversion() {
            let duration = Duration::from_millis(1500);
            let seconds = TimingUtils::duration_to_seconds(duration);
            assert_eq!(seconds, 1.5);
        }

        #[test]
        fn should_report_time_checks() {
            let cli_with_timing = Cli {
                report_time: Some(super::super::cli::ReportTime::Plain),
                ..Default::default()
            };
            let cli_without_timing = Cli {
                report_time: None,
                ..Default::default()
            };
            
            assert!(TimingUtils::should_report_time(&cli_with_timing));
            assert!(!TimingUtils::should_report_time(&cli_without_timing));
        }
    }

    mod background_utils_tests {
        use super::*;

        #[test]
        fn get_background_keyword_with_background() {
            let mut feature = mock_feature("Test", None);
            feature.background = Some(gherkin::Background {
                keyword: "Предистория".to_string(), // Russian background keyword
                description: None,
                steps: vec![],
                span: gherkin::Span { start: 0, end: 0 },
                position: gherkin::Position::new(1, 1),
            });
            
            let keyword = BackgroundUtils::get_background_keyword(&feature);
            assert_eq!(keyword, "Предистория");
        }

        #[test]
        fn get_background_keyword_without_background() {
            let feature = mock_feature("Test", None);
            let keyword = BackgroundUtils::get_background_keyword(&feature);
            assert_eq!(keyword, "Background");
        }

        #[test]
        fn format_background_step_name() {
            let feature = mock_feature("Test", None);
            let step = mock_step("Given", "some precondition", 5);
            
            let formatted = BackgroundUtils::format_background_step_name(&feature, &step);
            assert_eq!(formatted, "5: Background Given some precondition");
        }

        #[test]
        fn format_regular_step_name() {
            let step = mock_step("When", "user clicks button", 10);
            let formatted = BackgroundUtils::format_regular_step_name(&step);
            assert_eq!(formatted, "10: When user clicks button");
        }
    }
}