//! Simplified integration tests for writer module consolidation.

use cucumber::writer::WriterStats;

#[test]
fn writer_stats_integration_test() {
    let mut stats = WriterStats::new();
    
    // Simulate a test run
    stats.record_passed_step();
    stats.record_failed_step();
    stats.record_skipped_step();
    stats.record_retried_step();
    stats.record_parsing_error();
    stats.record_hook_error();
    
    // Verify all metrics are tracked correctly
    assert_eq!(stats.passed_steps, 1);
    assert_eq!(stats.failed_steps, 1);
    assert_eq!(stats.skipped_steps, 1);
    assert_eq!(stats.retried_steps, 1);
    assert_eq!(stats.parsing_errors, 1);
    assert_eq!(stats.hook_errors, 1);
    
    // Test computed properties
    assert_eq!(stats.total_steps(), 3); // passed + failed + skipped
    assert!(stats.execution_has_failed()); // due to failed step + errors
}

#[test]
fn writer_stats_copy_semantics() {
    let stats1 = WriterStats::new();
    let stats2 = stats1; // Should compile due to Copy trait
    
    assert_eq!(stats1.total_steps(), stats2.total_steps());
}

#[test]
fn writer_stats_default_behavior() {
    let stats1 = WriterStats::new();
    let stats2 = WriterStats::default();
    
    assert_eq!(stats1.passed_steps, stats2.passed_steps);
    assert_eq!(stats1.failed_steps, stats2.failed_steps);
    assert_eq!(stats1.total_steps(), stats2.total_steps());
    assert_eq!(stats1.execution_has_failed(), stats2.execution_has_failed());
}

#[test]
fn error_consolidation_works() {
    use cucumber::error::{WriterError, CucumberError};
    use std::io;
    
    // Test error conversion chain
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let writer_err = WriterError::from(io_err);
    let cucumber_err = CucumberError::from(writer_err);
    
    match cucumber_err {
        CucumberError::Writer(WriterError::Io(inner)) => {
            assert_eq!(inner.kind(), io::ErrorKind::PermissionDenied);
        }
        _ => panic!("Expected Writer(Io(_)) error"),
    }
}

#[test]  
fn shared_utilities_are_accessible() {
    use cucumber::writer::{WorldFormatter, ErrorFormatter, CommonWriterExt};
    
    // Test WorldFormatter
    let world = Some(&42);
    let result = WorldFormatter::format_world_if_needed(world, cucumber::writer::Verbosity::ShowWorld);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), "42");
    
    // Test ErrorFormatter
    let error = std::io::Error::new(std::io::ErrorKind::NotFound, "test error");
    let formatted = ErrorFormatter::format_with_context(&error, "Test context");
    assert!(formatted.contains("Test context"));
    assert!(formatted.contains("test error"));
    
    // Test CommonWriterExt (should not panic)
    let result: Result<(), &str> = Ok(());
    result.handle_write_error("test context");
    
    let err_result: Result<(), &str> = Err("test error");
    err_result.handle_write_error("test context"); // Should print warning, not panic
}

#[test]
fn verbosity_levels_work() {
    use cucumber::writer::Verbosity;
    
    assert!(!Verbosity::Default.shows_world());
    assert!(!Verbosity::Default.shows_docstring());
    
    assert!(Verbosity::ShowWorld.shows_world());
    assert!(!Verbosity::ShowWorld.shows_docstring());
    
    assert!(Verbosity::ShowWorldAndDocString.shows_world());
    assert!(Verbosity::ShowWorldAndDocString.shows_docstring());
}

#[test]
fn world_formatter_handles_different_types() {
    use cucumber::writer::{WorldFormatter, Verbosity};
    
    // Test with different world types
    let int_world = Some(&42i32);
    let string_world = Some(&"test_world");
    let test_vec = vec![1, 2, 3];
    let struct_world = Some(&test_vec);
    
    // Should format all types with ShowWorld verbosity
    assert!(WorldFormatter::format_world_if_needed(int_world, Verbosity::ShowWorld).is_some());
    assert!(WorldFormatter::format_world_if_needed(string_world, Verbosity::ShowWorld).is_some());
    assert!(WorldFormatter::format_world_if_needed(struct_world, Verbosity::ShowWorld).is_some());
    
    // Should return None with Default verbosity
    assert!(WorldFormatter::format_world_if_needed(int_world, Verbosity::Default).is_none());
    assert!(WorldFormatter::format_world_if_needed(string_world, Verbosity::Default).is_none());
    assert!(WorldFormatter::format_world_if_needed(struct_world, Verbosity::Default).is_none());
}

#[test]
fn error_formatter_handles_different_panic_types() {
    use cucumber::writer::ErrorFormatter;
    use std::sync::Arc;
    
    // Test with String panic
    let string_panic: Arc<dyn std::any::Any + Send> = Arc::new("string panic".to_string());
    let formatted = ErrorFormatter::format_panic_message(&string_panic);
    assert_eq!(formatted, "string panic");
    
    // Test with &str panic
    let str_panic: Arc<dyn std::any::Any + Send> = Arc::new("str panic");
    let formatted = ErrorFormatter::format_panic_message(&str_panic);
    assert_eq!(formatted, "str panic");
    
    // Test with unknown type panic
    let unknown_panic: Arc<dyn std::any::Any + Send> = Arc::new(42i32);
    let formatted = ErrorFormatter::format_panic_message(&unknown_panic);
    assert_eq!(formatted, "Unknown error");
}

#[test]
fn writer_consolidation_memory_efficiency() {
    use std::mem;
    
    // WriterStats should be small and efficient
    let stats = WriterStats::new();
    let size = mem::size_of_val(&stats);
    
    // Should be 6 usizes (one for each counter field)
    assert_eq!(size, 6 * mem::size_of::<usize>());
    
    // Should be Copy (compile-time check)
    let _stats2 = stats; // This compiles because WriterStats is Copy
}

#[test]
fn consolidation_reduces_complexity() {
    // This test demonstrates that the consolidation provides
    // a unified interface for common writer operations
    
    let mut stats = WriterStats::new();
    
    // Before consolidation: each writer implemented its own stats tracking
    // After consolidation: all writers can use WriterStats
    
    // Simulate complex test execution with multiple event types
    for i in 0..10 {
        if i % 3 == 0 {
            stats.record_passed_step();
        } else if i % 3 == 1 {
            stats.record_failed_step();
        } else {
            stats.record_skipped_step();
        }
        
        if i % 5 == 0 {
            stats.record_retried_step();
        }
    }
    
    stats.record_parsing_error();
    stats.record_hook_error();
    
    // Verify comprehensive tracking
    assert_eq!(stats.passed_steps, 4);   // 0, 3, 6, 9
    assert_eq!(stats.failed_steps, 3);   // 1, 4, 7
    assert_eq!(stats.skipped_steps, 3);  // 2, 5, 8
    assert_eq!(stats.retried_steps, 2);  // 0, 5
    assert_eq!(stats.parsing_errors, 1);
    assert_eq!(stats.hook_errors, 1);
    assert_eq!(stats.total_steps(), 10);
    assert!(stats.execution_has_failed());
}