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
//! This module has been refactored into a modular structure for better
//! maintainability and Single Responsibility Principle compliance.
//! All functionality remains backward compatible through re-exports.
//!
//! [1]: https://doc.rust-lang.org/rustc/tests/index.html

mod libtest;

// Re-export all public items for backward compatibility
pub use libtest::{
    // Core types
    Libtest, Or, OrBasic,
    
    // CLI types
    Cli, Format, ReportTime,
    
    // JSON event types
    LibTestJsonEvent, SuiteEvent, SuiteResults, TestEvent, TestEventInner,
    
    // Utility types
    IsBackground, LibtestUtils, TimingUtils, BackgroundUtils,
};

// Additional re-exports for complete backward compatibility
pub use libtest::writer::Libtest as LibtestWriter;

#[cfg(test)]
mod backward_compatibility_tests {
    use super::*;
    use crate::{World, Writer};
    use std::io;

    #[derive(Debug)]
    struct MockWorld;
    impl World for MockWorld {}

    #[test]
    fn all_original_types_accessible() {
        // Test that all original public types are still accessible
        let _cli: Cli = Cli::default();
        let _format: Format = Format::Json;
        let _report_time: ReportTime = ReportTime::Plain;
        let _libtest: Libtest<MockWorld, Vec<u8>> = Libtest::raw(Vec::new());
        let _suite_event: SuiteEvent = SuiteEvent::Started { test_count: 5 };
        let _test_event: TestEvent = TestEvent::started("test".to_string());
        let _suite_results: SuiteResults = SuiteResults {
            passed: 1,
            failed: 0,
            ignored: 0,
            measured: 0,
            filtered_out: 0,
            exec_time: None,
        };
        let _test_event_inner: TestEventInner = TestEventInner::new("test".to_string());
        let _json_event: LibTestJsonEvent = _suite_event.into();
    }

    #[test]
    fn libtest_constructors_work() {
        // Test original constructor methods
        let _raw_writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        let _new_writer = Libtest::<MockWorld, Vec<u8>>::new(Vec::new());
        let _stdout_writer = Libtest::<MockWorld>::stdout();
        let _or_basic_writer = Libtest::<MockWorld>::or_basic();
    }

    #[test]
    fn writer_trait_still_implemented() {
        // Test that Writer trait is still correctly implemented
        fn assert_is_writer<W: Writer<MockWorld>>(_: &W) {}
        
        let writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        assert_is_writer(&writer);
        
        // Check CLI type
        assert!(std::any::type_name::<<Libtest<MockWorld, Vec<u8>> as Writer<MockWorld>>::Cli>()
            .contains("Cli"));
    }

    #[test]
    fn stats_trait_still_implemented() {
        use crate::writer::Stats;
        
        let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        writer.passed = 5;
        writer.failed = 2;
        writer.ignored = 1;
        writer.retried = 1;
        writer.parsing_errors = 1;
        writer.hook_errors = 1;
        
        assert_eq!(writer.passed_steps(), 5);
        assert_eq!(writer.failed_steps(), 2);
        assert_eq!(writer.skipped_steps(), 1);
        assert_eq!(writer.retried_steps(), 1);
        assert_eq!(writer.parsing_errors(), 1);
        assert_eq!(writer.hook_errors(), 1);
    }

    #[test]
    fn arbitrary_trait_still_implemented() {
        use crate::writer::Arbitrary;
        
        fn assert_is_arbitrary<W, V>(_: &dyn Arbitrary<W, V>) {}
        
        let writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        assert_is_arbitrary(&writer as &dyn Arbitrary<MockWorld, &str>);
    }

    #[test]
    fn output_formatter_trait_still_implemented() {
        use crate::writer::common::OutputFormatter;
        
        let mut writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        let _output = writer.output_mut();
    }

    #[test]
    fn non_transforming_trait_still_implemented() {
        use crate::writer::NonTransforming;
        
        fn assert_is_non_transforming<W>(_: &dyn NonTransforming) {}
        
        let writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        assert_is_non_transforming(&writer);
    }

    #[test]
    fn json_serialization_still_works() {
        let suite_event = SuiteEvent::Started { test_count: 10 };
        let json_event: LibTestJsonEvent = suite_event.into();
        
        let serialized = serde_json::to_string(&json_event)
            .expect("Should serialize");
        
        assert!(serialized.contains("\"type\":\"suite\""));
        assert!(serialized.contains("\"event\":\"started\""));
    }

    #[test]
    fn clone_still_works() {
        let writer1 = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        let _writer2 = writer1.clone();
    }

    #[test]
    fn debug_still_works() {
        let writer = Libtest::<MockWorld, Vec<u8>>::raw(Vec::new());
        let debug_str = format!("{writer:?}");
        assert!(debug_str.contains("Libtest"));
    }

    #[test]
    fn cli_from_str_still_works() {
        use std::str::FromStr;
        
        let format = Format::from_str("json").expect("Should parse");
        assert!(matches!(format, Format::Json));
        
        let report_time = ReportTime::from_str("plain").expect("Should parse");
        assert!(matches!(report_time, ReportTime::Plain));
    }

    #[test]
    fn type_aliases_still_work() {
        use crate::writer;
        
        // Test type aliases
        let _or_type: Or<MockWorld, writer::Basic> = 
            Libtest::or(writer::Basic::raw(Vec::new()));
        let _or_basic_type: OrBasic<MockWorld> = Libtest::or_basic();
    }
}