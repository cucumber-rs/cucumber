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
//! This module provides a comprehensive error handling system organized by
//! responsibility and error domain. Each submodule focuses on a specific
//! area of error handling while maintaining full backward compatibility.
//!
//! # Modules
//!
//! - [`core`] - Main [`CucumberError`] type and core functionality
//! - [`step`] - Step execution errors and utilities  
//! - [`world`] - World trait implementation errors
//! - [`writer`] - Output writing and formatting errors
//! - [`config`] - Configuration and validation errors
//! - [`utilities`] - Shared traits and utility functions
//!
//! # Example
//!
//! ```rust
//! use cucumber::error::{CucumberError, StepError, Result};
//!
//! fn example_function() -> Result<()> {
//!     // This will automatically convert StepError to CucumberError
//!     Err(StepError::no_match("Given undefined step"))?
//! }
//! ```

pub mod config;
pub mod core;
pub mod step;
pub mod utilities;
pub mod world;
pub mod writer;

// Re-export all error types for backward compatibility
pub use config::{ConfigError, ConfigResult};
pub use core::{CucumberError, Result};
pub use step::{PanicPayloadExt, StepError, StepResult};
pub use utilities::{ResultExt, ResultConfigExt};
pub use world::{WorldError, WorldResult};
pub use writer::{WriterError, WriterResult};

// Additional result type aliases for convenience
/// Alias for the main cucumber Result type.
pub type CucumberResult<T> = Result<T>;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::{error::Error, io, sync::Arc};

    #[test]
    fn test_error_conversions() {
        // Test StepError to CucumberError conversion
        let step_err = StepError::no_match("Given undefined step");
        let cucumber_err: CucumberError = step_err.into();
        assert!(matches!(cucumber_err, CucumberError::Step(_)));

        // Test WorldError to CucumberError conversion
        let world_err = WorldError::invalid_state("missing field");
        let cucumber_err: CucumberError = world_err.into();
        assert!(matches!(cucumber_err, CucumberError::World(_)));

        // Test WriterError to CucumberError conversion
        let writer_err = WriterError::unavailable("buffer full");
        let cucumber_err: CucumberError = writer_err.into();
        assert!(matches!(cucumber_err, CucumberError::Writer(_)));

        // Test ConfigError to CucumberError conversion
        let config_err = ConfigError::invalid_retry("negative count");
        let cucumber_err: CucumberError = config_err.into();
        assert!(matches!(cucumber_err, CucumberError::Config(_)));

        // Test io::Error to CucumberError conversion
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let cucumber_err: CucumberError = io_err.into();
        assert!(matches!(cucumber_err, CucumberError::Io(_)));
    }

    #[test]
    fn test_convenience_constructors() {
        // Test CucumberError convenience methods
        let panic_err = CucumberError::step_panic("test panic");
        assert!(panic_err.to_string().contains("Step panicked: test panic"));

        let no_match_err = CucumberError::no_step_match("Given undefined");
        assert!(no_match_err.to_string().contains("No matching step found"));

        let ambiguous_err = CucumberError::ambiguous_step("Given ambiguous", 3);
        assert!(ambiguous_err.to_string().contains("matches 3 step definitions"));

        let world_creation_err = CucumberError::world_creation(
            io::Error::new(io::ErrorKind::Other, "creation failed")
        );
        assert!(world_creation_err.to_string().contains("Failed to create World"));

        let retry_config_err = CucumberError::invalid_retry_config("negative count");
        assert!(retry_config_err.to_string().contains("Invalid retry configuration"));
    }

    #[test]
    fn test_panic_payload_extension() {
        let string_payload: Arc<dyn std::any::Any + Send + 'static> = 
            Arc::new("test panic".to_string());
        assert_eq!(string_payload.to_readable_string(), "test panic");

        let str_payload: Arc<dyn std::any::Any + Send + 'static> = 
            Arc::new("test &str");
        assert_eq!(str_payload.to_readable_string(), "test &str");

        let unknown_payload: Arc<dyn std::any::Any + Send + 'static> = 
            Arc::new(42i32);
        assert_eq!(unknown_payload.to_readable_string(), "Unknown panic payload");
    }

    #[test]
    fn test_result_extensions() {
        use utilities::ResultExt;

        let ok_result: std::result::Result<i32, io::Error> = Ok(42);
        let cucumber_result = ok_result.with_cucumber_context("test context");
        assert!(cucumber_result.is_ok());
        assert_eq!(cucumber_result.unwrap(), 42);

        let err_result: std::result::Result<i32, io::Error> = 
            Err(io::Error::new(io::ErrorKind::Other, "test error"));
        let cucumber_result = err_result.with_cucumber_context("test context");
        assert!(cucumber_result.is_err());
        let err = cucumber_result.unwrap_err();
        assert!(err.to_string().contains("test context"));
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_all_result_types() {
        // Test that all result types are properly exported and usable
        let _cucumber_result: Result<()> = Ok(());
        let _cucumber_result_alias: CucumberResult<()> = Ok(());
        let _step_result: StepResult<()> = Ok(());
        let _world_result: WorldResult<()> = Ok(());
        let _writer_result: WriterResult<()> = Ok(());
        let _config_result: ConfigResult<()> = Ok(());
    }

    #[test]
    fn test_error_source_chains() {
        // Test that error source chains work properly across modules
        let io_err = io::Error::new(io::ErrorKind::Other, "root cause");
        let writer_err = WriterError::Io(io_err);
        let cucumber_err = CucumberError::Writer(writer_err);

        // Verify the error chain is preserved
        assert!(cucumber_err.source().is_some());
        if let Some(source) = cucumber_err.source() {
            assert!(source.to_string().contains("I/O error"));
            if let Some(root_source) = source.source() {
                assert!(root_source.to_string().contains("root cause"));
            }
        }
    }

    #[test]
    fn test_error_display_formatting() {
        // Test that all error types have proper display formatting
        let step_err = StepError::timeout("Given I wait", std::time::Duration::from_secs(30));
        assert!(step_err.to_string().contains("Step timed out after"));

        let world_err = WorldError::invalid_state("missing field");
        assert!(world_err.to_string().contains("World is in invalid state"));

        let writer_err = WriterError::unavailable("buffer full");
        assert!(writer_err.to_string().contains("Output unavailable"));

        let config_err = ConfigError::feature_file_not_found("missing.feature");
        assert!(config_err.to_string().contains("Feature file not found"));

        let cucumber_err = CucumberError::Step(step_err);
        assert!(cucumber_err.to_string().contains("Step execution failed"));
    }

    #[test]
    fn test_backward_compatibility() {
        // Ensure all original functionality is still accessible through re-exports
        
        // Original error types
        let _: CucumberError = CucumberError::step_panic("test");
        let _: StepError = StepError::no_match("test");
        let _: WorldError = WorldError::invalid_state("test");
        let _: WriterError = WriterError::unavailable("test");
        let _: ConfigError = ConfigError::invalid_retry("test");

        // Original result types
        let _: Result<()> = Ok(());
        let _: StepResult<()> = Ok(());
        let _: WorldResult<()> = Ok(());
        let _: WriterResult<()> = Ok(());
        let _: ConfigResult<()> = Ok(());

        // Original traits and extensions
        let result: std::result::Result<i32, io::Error> = Ok(42);
        let _: Result<i32> = result.with_cucumber_context("test");
    }
}