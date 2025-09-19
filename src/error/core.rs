// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Core error types for the Cucumber crate.
//!
//! This module contains the main [`CucumberError`] enum that consolidates
//! all error types from various Cucumber operations into a single hierarchy.

use std::io;

use derive_more::with_trait::{Display, Error};

use super::{ConfigError, StepError, WorldError, WriterError};

/// Top-level error type for all Cucumber operations.
///
/// This consolidates various error types that can occur during Cucumber
/// execution into a single, structured hierarchy.
#[derive(Debug, Display, Error)]
pub enum CucumberError {
    /// Error during feature file parsing.
    #[display("Failed to parse feature file: {_0}")]
    Parse(gherkin::ParseFileError),

    /// I/O error during file operations or output writing.
    #[display("I/O operation failed: {_0}")]
    Io(io::Error),

    /// Error during step execution.
    #[display("Step execution failed: {_0}")]
    Step(StepError),

    /// Error during world initialization.
    #[display("World initialization failed: {_0}")]
    World(WorldError),

    /// Writer-specific errors.
    #[display("Writer error: {_0}")]
    Writer(WriterError),

    /// Configuration or validation errors.
    #[display("Configuration error: {_0}")]
    Config(ConfigError),
}

/// Result type alias using [`CucumberError`].
pub type Result<T> = std::result::Result<T, CucumberError>;

impl CucumberError {
    /// Creates a step panic error.
    #[must_use]
    pub fn step_panic(message: impl Into<String>) -> Self {
        Self::Step(StepError::Panic {
            message: message.into(),
            payload: None,
        })
    }

    /// Creates a step panic error with payload.
    #[must_use]
    pub fn step_panic_with_payload(
        message: impl Into<String>,
        payload: std::sync::Arc<dyn std::any::Any + Send + 'static>,
    ) -> Self {
        Self::Step(StepError::Panic {
            message: message.into(),
            payload: Some(payload),
        })
    }

    /// Creates a no matching step error.
    #[must_use]
    pub fn no_step_match(step_text: impl Into<String>) -> Self {
        Self::Step(StepError::NoMatch {
            step_text: step_text.into(),
        })
    }

    /// Creates an ambiguous step error.
    #[must_use]
    pub fn ambiguous_step(step_text: impl Into<String>, count: usize) -> Self {
        Self::Step(StepError::Ambiguous {
            step_text: step_text.into(),
            count,
        })
    }

    /// Creates a world creation error.
    #[must_use]
    pub fn world_creation(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::World(WorldError::Creation {
            source: Box::new(source),
        })
    }

    /// Creates a configuration error for invalid retry settings.
    #[must_use]
    pub fn invalid_retry_config(reason: impl Into<String>) -> Self {
        Self::Config(ConfigError::InvalidRetry {
            reason: reason.into(),
        })
    }
}

impl From<gherkin::ParseFileError> for CucumberError {
    fn from(err: gherkin::ParseFileError) -> Self {
        Self::Parse(err)
    }
}

impl From<io::Error> for CucumberError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<WriterError> for CucumberError {
    fn from(err: WriterError) -> Self {
        Self::Writer(err)
    }
}

impl From<StepError> for CucumberError {
    fn from(err: StepError) -> Self {
        Self::Step(err)
    }
}

impl From<WorldError> for CucumberError {
    fn from(err: WorldError) -> Self {
        Self::World(err)
    }
}

impl From<ConfigError> for CucumberError {
    fn from(err: ConfigError) -> Self {
        Self::Config(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::sync::Arc;

    #[test]
    fn test_cucumber_error_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let cucumber_err = CucumberError::Io(io_err);
        assert!(cucumber_err.to_string().contains("I/O operation failed"));
    }

    #[test]
    fn test_step_panic_error() {
        let err = CucumberError::step_panic("test panic message");
        assert!(err.to_string().contains("Step execution failed"));
        assert!(err.to_string().contains("Step panicked: test panic message"));
        
        if let CucumberError::Step(StepError::Panic { message, payload }) = err {
            assert_eq!(message, "test panic message");
            assert!(payload.is_none());
        } else {
            panic!("Expected step panic error");
        }
    }

    #[test]
    fn test_step_panic_with_payload() {
        let payload: Arc<dyn std::any::Any + Send + 'static> = Arc::new("panic payload".to_string());
        let err = CucumberError::step_panic_with_payload("test panic", payload.clone());
        
        if let CucumberError::Step(StepError::Panic { message, payload: Some(_) }) = err {
            assert_eq!(message, "test panic");
        } else {
            panic!("Expected step panic error with payload");
        }
    }

    #[test]
    fn test_no_step_match_error() {
        let err = CucumberError::no_step_match("Given I do something");
        assert!(err.to_string().contains("Step execution failed"));
        assert!(err.to_string().contains("No matching step found for: Given I do something"));
    }

    #[test]
    fn test_ambiguous_step_error() {
        let err = CucumberError::ambiguous_step("Given I do something", 3);
        assert!(err.to_string().contains("Step execution failed"));
        assert!(err.to_string().contains("Ambiguous step: Given I do something matches 3 step definitions"));
    }

    #[test]
    fn test_world_creation_error() {
        let source_err = io::Error::new(io::ErrorKind::Other, "world creation failed");
        let err = CucumberError::world_creation(source_err);
        assert!(err.to_string().contains("World initialization failed"));
        assert!(err.to_string().contains("Failed to create World"));
    }

    #[test]
    fn test_invalid_retry_config_error() {
        let err = CucumberError::invalid_retry_config("negative retry count");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("Invalid retry configuration: negative retry count"));
    }

    #[test]
    fn test_from_conversions() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let cucumber_err: CucumberError = io_err.into();
        assert!(matches!(cucumber_err, CucumberError::Io(_)));

        let writer_err = WriterError::Format(std::fmt::Error);
        let cucumber_err: CucumberError = writer_err.into();
        assert!(matches!(cucumber_err, CucumberError::Writer(_)));
    }

    #[test]
    fn test_error_source_chain() {
        let io_err = io::Error::new(io::ErrorKind::Other, "root cause");
        let writer_err = WriterError::Io(io_err);
        let cucumber_err = CucumberError::Writer(writer_err);

        assert!(cucumber_err.source().is_some());
        
        if let Some(source) = cucumber_err.source() {
            assert!(source.to_string().contains("I/O error"));
            if let Some(root_source) = source.source() {
                assert!(root_source.to_string().contains("root cause"));
            }
        }
    }
}