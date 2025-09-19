// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Rust `libtest`][1] compatible [`Writer`] implementation.
//!
//! This module provides a modular implementation of the libtest writer,
//! organized into separate modules for better maintainability:
//!
//! - [`cli`]: CLI configuration and options
//! - [`writer`]: Core writer structure and implementation
//! - [`event_handlers`]: Event handling logic
//! - [`json_events`]: JSON event type definitions
//! - [`utils`]: Utility functions for formatting and timing
//!
//! [1]: https://doc.rust-lang.org/rustc/tests/index.html

pub mod cli;
pub mod event_handlers;
pub mod json_events;
pub mod utils;
pub mod writer;

// Re-export all public types for backward compatibility
pub use cli::{Cli, Format, ReportTime};
pub use json_events::{LibTestJsonEvent, SuiteEvent, SuiteResults, TestEvent, TestEventInner};
pub use utils::{IsBackground, LibtestUtils, TimingUtils, BackgroundUtils};
pub use writer::{Libtest, Or, OrBasic};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{Event, World, Writer, event};
    use std::{io, time::SystemTime};

    #[derive(Debug)]
    struct MockWorld;
    impl World for MockWorld {}

    #[tokio::test]
    async fn libtest_writer_integration() {
        let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        let cli = Cli::default();

        // Test basic event handling
        let meta = event::Metadata::new(SystemTime::now());
        let started_event = Ok(meta.insert(event::Cucumber::Started));
        
        writer.handle_event(started_event, &cli).await;
        
        // Before parsing finished, no output should be generated
        assert!(writer.output.is_empty());
        assert_eq!(writer.events.len(), 1);
    }

    #[test]
    fn libtest_writer_stats_integration() {
        let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        
        // Simulate some test execution
        writer.passed = 5;
        writer.failed = 2;
        writer.ignored = 1;
        writer.retried = 1;
        writer.parsing_errors = 1;
        writer.hook_errors = 0;
        
        // Test stats trait implementation
        use crate::writer::Stats;
        assert_eq!(writer.passed_steps(), 5);
        assert_eq!(writer.failed_steps(), 2);
        assert_eq!(writer.skipped_steps(), 1);
        assert_eq!(writer.retried_steps(), 1);
        assert_eq!(writer.parsing_errors(), 1);
        assert_eq!(writer.hook_errors(), 0);
    }

    #[test]
    fn cli_format_integration() {
        let cli = Cli {
            format: Some(Format::Json),
            show_output: true,
            report_time: Some(ReportTime::Colored),
            nightly: None,
        };
        
        // Test that CLI options work as expected
        assert!(matches!(cli.format, Some(Format::Json)));
        assert!(cli.show_output);
        assert!(matches!(cli.report_time, Some(ReportTime::Colored)));
    }

    #[test]
    fn json_events_integration() {
        // Test event creation and serialization
        let suite_event = SuiteEvent::Started { test_count: 10 };
        let json_event: LibTestJsonEvent = suite_event.into();
        
        let serialized = serde_json::to_string(&json_event)
            .expect("Should serialize successfully");
        
        assert!(serialized.contains("\"type\":\"suite\""));
        assert!(serialized.contains("\"event\":\"started\""));
        assert!(serialized.contains("\"test_count\":10"));
    }

    #[test]
    fn utils_integration() {
        use std::path::PathBuf;
        
        // Test utility functions work together
        let feature = gherkin::Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::Position::new(1, 1),
            path: Some(PathBuf::from("test.feature")),
        };
        
        let formatted_path = LibtestUtils::format_feature_path(&feature);
        assert_eq!(formatted_path, "test.feature");
        
        let cli = Cli {
            report_time: Some(ReportTime::Plain),
            ..Default::default()
        };
        assert!(TimingUtils::should_report_time(&cli));
    }

    #[test]
    fn backward_compatibility_types() {
        // Ensure all types are still accessible at the module level
        let _cli: Cli = Cli::default();
        let _format: Format = Format::Json;
        let _report_time: ReportTime = ReportTime::Plain;
        let _suite_event: SuiteEvent = SuiteEvent::Started { test_count: 0 };
        let _test_event: TestEvent = TestEvent::started("test".to_string());
        let _writer: Libtest<MockWorld, Vec<u8>> = Libtest::raw(Vec::new());
    }

    #[test]
    fn module_organization_test() {
        // Test that all modules are properly organized and accessible
        
        // CLI module
        let _cli_item = cli::Cli::default();
        
        // JSON events module
        let _json_item = json_events::TestEvent::started("test".to_string());
        
        // Utils module
        let _background_keyword = utils::BackgroundUtils::get_background_keyword;
        
        // Writer module
        let _writer_item = writer::Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
    }

    #[tokio::test]
    async fn full_workflow_integration() {
        let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        let cli = Cli {
            format: Some(Format::Json),
            show_output: false,
            report_time: Some(ReportTime::Plain),
            nightly: None,
        };

        // Simulate a complete workflow
        
        // 1. Start cucumber
        let meta = event::Metadata::new(SystemTime::now());
        let started_event = Ok(meta.insert(event::Cucumber::Started));
        writer.handle_event(started_event, &cli).await;
        
        // 2. Parsing finished
        let parsing_finished = Ok(meta.insert(event::Cucumber::ParsingFinished {
            steps: 5,
            parser_errors: 0,
            features: 1,
        }));
        writer.handle_event(parsing_finished, &cli).await;
        
        // 3. Finish cucumber
        let finished_event = Ok(meta.insert(event::Cucumber::Finished));
        writer.handle_event(finished_event, &cli).await;
        
        // Should have generated JSON output
        assert!(!writer.output.is_empty());
        let output_str = String::from_utf8_lossy(&writer.output);
        
        // Should contain suite events
        assert!(output_str.contains("\"type\":\"suite\""));
        assert!(output_str.contains("\"event\":\"started\""));
    }
}