//! Edge case and error handling tests for writer consolidation.

use cucumber::writer::{WriterStats, OutputFormatter, CommonWriterExt};
use cucumber::error::WriterError;
use std::io::{self, Write};

// Test various edge cases and error conditions

/// Mock writer that can simulate different failure modes
struct FailingWriter {
    buffer: Vec<u8>,
    fail_mode: FailMode,
    call_count: usize,
}

#[derive(Clone, Copy)]
enum FailMode {
    Never,
    Always,
    AfterNCalls(usize),
    OnSpecificCall(usize),
}

impl FailingWriter {
    fn new(fail_mode: FailMode) -> Self {
        Self {
            buffer: Vec::new(),
            fail_mode,
            call_count: 0,
        }
    }

    fn should_fail(&mut self) -> bool {
        self.call_count += 1;
        match self.fail_mode {
            FailMode::Never => false,
            FailMode::Always => true,
            FailMode::AfterNCalls(n) => self.call_count > n,
            FailMode::OnSpecificCall(n) => self.call_count == n,
        }
    }

    fn written_content(&self) -> String {
        String::from_utf8_lossy(&self.buffer).to_string()
    }
}

impl Write for FailingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.should_fail() {
            Err(io::Error::new(io::ErrorKind::WriteZero, "simulated failure"))
        } else {
            self.buffer.extend_from_slice(buf);
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.should_fail() {
            Err(io::Error::new(io::ErrorKind::WriteZero, "simulated flush failure"))
        } else {
            Ok(())
        }
    }
}

impl OutputFormatter for FailingWriter {
    type Output = Self;

    fn output_mut(&mut self) -> &mut Self::Output {
        self
    }
}

#[test]
fn output_formatter_handles_partial_failures() {
    let mut writer = FailingWriter::new(FailMode::OnSpecificCall(3));
    
    // First write should succeed
    assert!(writer.write_line("line 1").is_ok());
    
    // Second write should fail
    assert!(writer.write_line("line 2").is_err());
    
    // Content should contain only the first line
    let content = writer.written_content();
    assert!(content.contains("line 1"));
    assert!(!content.contains("line 2"));
}

#[test]
fn output_formatter_write_bytes_edge_cases() {
    let mut writer = FailingWriter::new(FailMode::Never);
    
    // Empty bytes
    assert!(writer.write_bytes(b"").is_ok());
    
    // Large bytes
    let large_data = vec![b'A'; 10_000];
    assert!(writer.write_bytes(&large_data).is_ok());
    
    // Non-UTF8 bytes
    let binary_data = vec![0xFF, 0xFE, 0xFD];
    assert!(writer.write_bytes(&binary_data).is_ok());
    
    let content = writer.buffer;
    assert_eq!(content.len(), 10_003); // empty + large + binary
}

#[test]
fn output_formatter_write_fmt_edge_cases() {
    let mut writer = FailingWriter::new(FailMode::Never);
    
    // Empty format
    assert!(OutputFormatter::write_fmt(&mut writer, format_args!("")).is_ok());
    
    // Complex formatting
    assert!(OutputFormatter::write_fmt(&mut writer, format_args!("{:?} {:x} {:.2}", vec![1,2,3], 255, 3.14159)).is_ok());
    
    // Unicode content
    assert!(OutputFormatter::write_fmt(&mut writer, format_args!("🚀 Unicode test 中文")).is_ok());
    
    let content = writer.written_content();
    assert!(content.contains("[1, 2, 3]"));
    assert!(content.contains("ff"));
    assert!(content.contains("3.14"));
    assert!(content.contains("🚀"));
    assert!(content.contains("中文"));
}

#[test]
fn writer_stats_extreme_values() {
    let mut stats = WriterStats::new();
    
    // Add a lot of each type
    for _ in 0..1000 {
        stats.record_passed_step();
        stats.record_failed_step();
        stats.record_skipped_step();
        stats.record_retried_step();
        stats.record_parsing_error();
        stats.record_hook_error();
    }
    
    assert_eq!(stats.passed_steps, 1000);
    assert_eq!(stats.failed_steps, 1000);
    assert_eq!(stats.skipped_steps, 1000);
    assert_eq!(stats.retried_steps, 1000);
    assert_eq!(stats.parsing_errors, 1000);
    assert_eq!(stats.hook_errors, 1000);
    assert_eq!(stats.total_steps(), 3000);
    assert!(stats.execution_has_failed());
}

#[test]
fn writer_stats_overflow_protection() {
    let mut stats = WriterStats::new();
    
    // Set to near max value to test overflow behavior
    stats.passed_steps = usize::MAX / 3;
    stats.failed_steps = usize::MAX / 3;  
    stats.skipped_steps = usize::MAX / 3;
    
    // This should not panic and should return a valid total
    let total = stats.total_steps();
    
    // Should be close to MAX but not overflow
    assert!(total > 0);
    assert!(total >= stats.passed_steps);
    assert!(total >= stats.failed_steps); 
    assert!(total >= stats.skipped_steps);
}

#[test]
fn writer_ext_error_handling_with_various_error_types() {
    // Test with different error types
    let io_result: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::PermissionDenied, "access denied"));
    io_result.handle_write_error("IO operation");
    
    let parse_result: Result<(), std::num::ParseIntError> = "not_a_number".parse::<i32>().map(|_| ());
    parse_result.handle_write_error("Parse operation");
    
    let custom_result: Result<(), &'static str> = Err("custom error");
    custom_result.handle_write_error("Custom operation");
    
    // All should handle errors gracefully without panicking
}

#[test] 
fn error_conversion_chain_testing() {
    // Test the full error conversion chain
    let io_err = io::Error::new(io::ErrorKind::UnexpectedEof, "unexpected EOF");
    let writer_err = WriterError::from(io_err);
    let cucumber_err = cucumber::error::CucumberError::from(writer_err);
    
    // Should be able to convert through the chain
    match cucumber_err {
        cucumber::error::CucumberError::Writer(WriterError::Io(inner)) => {
            assert_eq!(inner.kind(), io::ErrorKind::UnexpectedEof);
            assert!(inner.to_string().contains("unexpected EOF"));
        }
        _ => panic!("Expected Writer(Io(_)) error"),
    }
}

#[test]
fn format_error_conversion() {
    use std::fmt;
    
    let fmt_err = fmt::Error;
    let writer_err = WriterError::from(fmt_err);
    
    match writer_err {
        WriterError::Format(_) => {}, // Expected
        _ => panic!("Expected Format error"),
    }
}

#[cfg(feature = "output-json")]
#[test]
fn serde_json_error_conversion() {
    // Test serde_json error conversion if the feature is enabled
    let json_str = "{invalid json";
    let parse_result: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(json_str);
    
    if let Err(json_err) = parse_result {
        let writer_err = WriterError::from(json_err);
        match writer_err {
            WriterError::Serialization(_) => {}, // Expected
            _ => panic!("Expected Serialization error"),
        }
    }
}

#[test]
fn concurrent_writer_stats_usage() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    let stats = Arc::new(Mutex::new(WriterStats::new()));
    let mut handles = vec![];
    
    // Spawn multiple threads to modify stats
    for _ in 0..10 {
        let stats_clone = Arc::clone(&stats);
        let handle = thread::spawn(move || {
            let mut stats = stats_clone.lock().unwrap();
            for _ in 0..100 {
                stats.record_passed_step();
                stats.record_failed_step();
            }
        });
        handles.push(handle);
    }
    
    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
    
    let final_stats = stats.lock().unwrap();
    assert_eq!(final_stats.passed_steps, 1000);
    assert_eq!(final_stats.failed_steps, 1000);
    assert_eq!(final_stats.total_steps(), 2000);
    assert!(final_stats.execution_has_failed());
}

#[test]
fn writer_stats_update_from_step_event_edge_cases() {
    use cucumber::event;
    use cucumber::event::Retries;
    
    let mut stats = WriterStats::new();
    
    // Test with zero retries
    let retries_zero = Retries { left: 0, current: 3 };
    let event = event::Step::<i32>::Started;
    
    stats.update_from_step_event(&event, Some(&retries_zero));
    assert_eq!(stats.retried_steps, 0); // Should not increment
    
    // Test with maximum retries
    let retries_max = Retries { left: usize::MAX, current: 1 };
    stats.update_from_step_event(&event, Some(&retries_max));
    assert_eq!(stats.retried_steps, 1); // Should increment once
    
    // Test repeated calls
    for _ in 0..10 {
        stats.update_from_step_event(&event, Some(&retries_max));
    }
    assert_eq!(stats.retried_steps, 11); // Should increment each time
}

#[test]
fn memory_safety_with_large_outputs() {
    let mut writer = FailingWriter::new(FailMode::Never);
    
    // Test with very large strings
    let large_string = "A".repeat(100_000);
    assert!(writer.write_line(&large_string).is_ok());
    
    // Test with many small writes
    for i in 0..1000 {
        assert!(writer.write_line(&format!("line {}", i)).is_ok());
    }
    
    let content = writer.written_content();
    assert!(content.len() > 100_000);
    assert!(content.contains("line 999"));
}

#[test]
fn error_handling_resilience() {
    let mut writer = FailingWriter::new(FailMode::AfterNCalls(10));
    let mut successful_writes = 0;
    let mut failed_writes = 0;
    
    // Try to write many lines, some should fail
    for i in 0..20 {
        match writer.write_line(&format!("line {}", i)) {
            Ok(_) => successful_writes += 1,
            Err(_) => failed_writes += 1,
        }
    }
    
    assert_eq!(successful_writes, 5); // First 5 should succeed (writeln makes 2 calls each)
    assert_eq!(failed_writes, 15);    // Rest should fail
    
    // Content should only contain successful writes
    let content = writer.written_content();
    assert!(content.contains("line 0"));
    assert!(content.contains("line 4"));
    assert!(!content.contains("line 5")); // This and beyond should have failed
}

#[test]
fn writer_stats_edge_case_combinations() {
    let mut stats = WriterStats::new();
    
    // Test various combinations of success/failure
    stats.record_passed_step();
    assert!(!stats.execution_has_failed());
    
    stats.record_skipped_step(); 
    assert!(!stats.execution_has_failed()); // Still not failed
    
    stats.record_failed_step();
    assert!(stats.execution_has_failed()); // Now failed
    
    // Add more passed steps - should still be considered failed
    stats.record_passed_step();
    stats.record_passed_step();
    assert!(stats.execution_has_failed());
    
    // Test with just hook errors
    let mut stats2 = WriterStats::new();
    stats2.record_hook_error();
    assert!(stats2.execution_has_failed());
    
    // Test with just parsing errors
    let mut stats3 = WriterStats::new();
    stats3.record_parsing_error();
    assert!(stats3.execution_has_failed());
}