//! WriterStats edge case and stress tests.

use cucumber::writer::{WriterStats, CommonWriterExt};
use cucumber::event;
use cucumber::event::Retries;
use std::sync::{Arc, Mutex};
use std::thread;

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
fn concurrent_writer_stats_usage() {
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

#[test]
fn writer_stats_zero_values() {
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
fn writer_stats_max_values() {
    let mut stats = WriterStats::new();
    
    // Set all to max values
    stats.passed_steps = usize::MAX;
    stats.failed_steps = usize::MAX;
    stats.skipped_steps = usize::MAX;
    stats.retried_steps = usize::MAX;
    stats.parsing_errors = usize::MAX;
    stats.hook_errors = usize::MAX;
    
    // Should indicate failure
    assert!(stats.execution_has_failed());
    
    // Total should not panic (may overflow in debug mode but should handle gracefully)
    let _ = stats.total_steps();
}

#[test]
fn writer_stats_single_failure_types() {
    // Test each failure type individually
    let mut stats1 = WriterStats::new();
    stats1.record_failed_step();
    assert!(stats1.execution_has_failed());
    
    let mut stats2 = WriterStats::new();
    stats2.record_parsing_error();
    assert!(stats2.execution_has_failed());
    
    let mut stats3 = WriterStats::new();
    stats3.record_hook_error();
    assert!(stats3.execution_has_failed());
    
    // Non-failure types should not indicate failure
    let mut stats4 = WriterStats::new();
    stats4.record_passed_step();
    stats4.record_skipped_step();
    stats4.record_retried_step();
    assert!(!stats4.execution_has_failed());
}

#[test]
fn writer_stats_batch_operations() {
    let mut stats = WriterStats::new();
    
    // Test batch recording
    for _ in 0..100 {
        stats.record_passed_step();
    }
    for _ in 0..50 {
        stats.record_skipped_step();
    }
    for _ in 0..25 {
        stats.record_failed_step();
    }
    
    assert_eq!(stats.passed_steps, 100);
    assert_eq!(stats.skipped_steps, 50);
    assert_eq!(stats.failed_steps, 25);
    assert_eq!(stats.total_steps(), 175);
    assert!(stats.execution_has_failed());
}