// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Statistics tracking for writers.

use crate::{
    event::{self, Retries},
};

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
            event::Step::Passed { .. } => self.record_passed_step(),
            event::Step::Skipped => self.record_skipped_step(),
            event::Step::Failed { .. } => self.record_failed_step(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event;

    // Helper function to create a mock step event
    fn mock_step_event() -> event::Step<u32> {
        event::Step::Started
    }

    // Helper function to create mock retries
    fn mock_retries() -> Retries {
        Retries { left: 2, current: 1 }
    }

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
        let passed_event: event::Step<i32> = event::Step::Passed { captures, location: None };
        stats.update_from_step_event(&passed_event, Some(&retries));
        assert_eq!(stats.passed_steps, 1);
        assert_eq!(stats.retried_steps, 1); // Should record retry

        // Test failed step  
        let failed_event: event::Step<i32> = event::Step::Failed {
            captures: None,
            location: None,
            world: None,
            error: crate::event::StepError::NotFound
        };
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

    #[test]
    fn writer_stats_multiple_updates() {
        let mut stats = WriterStats::new();
        
        // Simulate multiple step executions
        for _ in 0..3 {
            stats.record_passed_step();
        }
        for _ in 0..2 {
            stats.record_failed_step();
        }
        for _ in 0..1 {
            stats.record_skipped_step();
        }
        
        assert_eq!(stats.passed_steps, 3);
        assert_eq!(stats.failed_steps, 2);
        assert_eq!(stats.skipped_steps, 1);
        assert_eq!(stats.total_steps(), 6);
        assert!(stats.execution_has_failed());
    }

    #[test]
    fn writer_stats_default_trait() {
        let stats = WriterStats::default();
        
        assert_eq!(stats.passed_steps, 0);
        assert_eq!(stats.failed_steps, 0);
        assert_eq!(stats.skipped_steps, 0);
        assert_eq!(stats.retried_steps, 0);
        assert_eq!(stats.parsing_errors, 0);
        assert_eq!(stats.hook_errors, 0);
    }
}