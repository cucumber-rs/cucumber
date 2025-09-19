// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Consolidated error handling types for the Cucumber crate.
//!
//! This module has been refactored into a modular structure while maintaining
//! full backward compatibility. All original types and functions are re-exported
//! from their respective specialized modules.
//!
//! For the new modular interface, use the `error::` submodules directly.

// Re-export everything from the new modular structure for backward compatibility
mod error;

pub use error::{
    config::{ConfigError, ConfigResult},
    core::{CucumberError, Result},
    step::{PanicPayloadExt, StepError, StepResult},
    utilities::{ResultExt, ResultConfigExt},
    world::{WorldError, WorldResult},
    writer::{WriterError, WriterResult},
    CucumberResult,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io, sync::Arc};

    /// Test backward compatibility - ensure all original functionality still works
    #[test]
    fn test_backward_compatibility() {
        // Test that all original error creation methods still work
        let _: CucumberError = CucumberError::step_panic("test");
        let _: CucumberError = CucumberError::no_step_match("Given undefined");
        let _: CucumberError = CucumberError::ambiguous_step("Given ambiguous", 2);
        let _: CucumberError = CucumberError::world_creation(
            io::Error::new(io::ErrorKind::Other, "test")
        );
        let _: CucumberError = CucumberError::invalid_retry_config("test");

        // Test that all result types are available
        let _: Result<()> = Ok(());
        let _: CucumberResult<()> = Ok(());
        let _: StepResult<()> = Ok(());
        let _: WorldResult<()> = Ok(());
        let _: WriterResult<()> = Ok(());
        let _: ConfigResult<()> = Ok(());

        // Test that traits work
        let result: std::result::Result<i32, io::Error> = Ok(42);
        let _: Result<i32> = result.with_cucumber_context("test");

        // Test panic payload extension
        let payload: Arc<dyn std::any::Any + Send + 'static> = Arc::new("test".to_string());
        assert_eq!(payload.to_readable_string(), "test");
    }

    /// Test that error conversions still work properly
    #[test]
    fn test_error_conversions() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let _: CucumberError = io_err.into();

        let writer_err = WriterError::unavailable("test");
        let _: CucumberError = writer_err.into();
    }

    /// Test that all error types can be created and displayed properly
    #[test]
    fn test_error_creation_and_display() {
        let step_err = StepError::no_match("Given undefined");
        assert!(step_err.to_string().contains("No matching step found"));

        let world_err = WorldError::invalid_state("test");
        assert!(world_err.to_string().contains("World is in invalid state"));

        let writer_err = WriterError::unavailable("buffer full");
        assert!(writer_err.to_string().contains("Output unavailable"));

        let config_err = ConfigError::invalid_retry("negative count");
        assert!(config_err.to_string().contains("Invalid retry configuration"));

        let cucumber_err = CucumberError::Step(step_err);
        assert!(cucumber_err.to_string().contains("Step execution failed"));
    }
}