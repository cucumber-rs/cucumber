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

use std::{fmt, io, sync::Arc};

use derive_more::with_trait::{Display, Error, From};

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

/// Errors that can occur during step execution.
#[derive(Debug, Display, Error)]
pub enum StepError {
    /// Step function panicked.
    #[display("Step panicked: {message}")]
    Panic {
        /// The panic message.
        #[error(not(source))]
        message: String,
        /// The panic payload if available.
        payload: Option<Arc<dyn std::any::Any + Send + 'static>>,
    },

    /// Step matching failed.
    #[display("No matching step found for: {step_text}")]
    NoMatch {
        /// The step text that couldn't be matched.
        #[error(not(source))]
        step_text: String,
    },

    /// Multiple steps matched.
    #[display("Ambiguous step: {step_text} matches {count} step definitions")]
    Ambiguous {
        /// The step text with multiple matches.
        #[error(not(source))]
        step_text: String,
        /// Number of matching step definitions.
        count: usize,
    },

    /// Step execution timed out.
    #[display("Step timed out after {duration:?}: {step_text}")]
    Timeout {
        /// The step text that timed out.
        #[error(not(source))]
        step_text: String,
        /// The timeout duration.
        duration: std::time::Duration,
    },
}

/// Errors related to World trait implementation.
#[derive(Debug, Display, Error)]
pub enum WorldError {
    /// Failed to create a new World instance.
    #[display("Failed to create World: {source}")]
    Creation {
        /// The underlying error from World::new().
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    /// World is in an invalid state.
    #[display("World is in invalid state: {reason}")]
    InvalidState {
        /// Reason for the invalid state.
        #[error(not(source))]
        reason: String,
    },
}

/// Writer-specific errors.
#[derive(Debug, Display, Error)]
pub enum WriterError {
    /// I/O error during output operations.
    #[display("I/O error: {_0}")]
    Io(io::Error),

    /// Failed to serialize output.
    #[cfg(any(feature = "output-json", feature = "libtest"))]
    #[display("Serialization failed: {_0}")]
    Serialization(serde_json::Error),

    /// Output formatting error.
    #[display("Format error: {_0}")]
    Format(fmt::Error),

    /// XML generation error (for JUnit output).
    #[cfg(feature = "output-junit")]
    #[display("XML generation failed: {_0}")]
    Xml(#[error(not(source))] String),

    /// Output buffer is full or unavailable.
    #[display("Output unavailable: {reason}")]
    Unavailable {
        /// Reason why output is unavailable.
        #[error(not(source))]
        reason: String,
    },
}

/// Configuration and validation errors.
#[derive(Debug, Display, Error)]
pub enum ConfigError {
    /// Invalid retry configuration.
    #[display("Invalid retry configuration: {reason}")]
    InvalidRetry {
        /// Reason for the invalid retry config.
        #[error(not(source))]
        reason: String,
    },

    /// Invalid tag filter expression.
    #[display("Invalid tag filter: {expression}")]
    InvalidTagFilter {
        /// The invalid tag expression.
        #[error(not(source))]
        expression: String,
    },

    /// Feature file not found or inaccessible.
    #[display("Feature file not found: {path}")]
    FeatureFileNotFound {
        /// The path that couldn't be found.
        #[error(not(source))]
        path: String,
    },

    /// Invalid CLI argument combination.
    #[display("Invalid CLI arguments: {reason}")]
    InvalidCliArgs {
        /// Reason for the invalid arguments.
        #[error(not(source))]
        reason: String,
    },
}

/// Result type alias using [`CucumberError`].
pub type Result<T> = std::result::Result<T, CucumberError>;

/// Result type alias for step operations.
pub type StepResult<T> = std::result::Result<T, StepError>;

/// Result type alias for world operations.
pub type WorldResult<T> = std::result::Result<T, WorldError>;

/// Result type alias for writer operations.
pub type WriterResult<T> = std::result::Result<T, WriterError>;

/// Result type alias for configuration operations.
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

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
        payload: Arc<dyn std::any::Any + Send + 'static>,
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

/// Trait for converting panic payloads to readable error messages.
pub trait PanicPayloadExt {
    /// Converts panic payload to a readable string.
    fn to_readable_string(&self) -> String;
}

impl PanicPayloadExt for Arc<dyn std::any::Any + Send + 'static> {
    fn to_readable_string(&self) -> String {
        if let Some(s) = self.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = self.downcast_ref::<&str>() {
            (*s).to_string()
        } else {
            "Unknown panic payload".to_string()
        }
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

impl From<io::Error> for WriterError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<fmt::Error> for WriterError {
    fn from(err: fmt::Error) -> Self {
        Self::Format(err)
    }
}

#[cfg(any(feature = "output-json", feature = "libtest"))]
impl From<serde_json::Error> for WriterError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err)
    }
}

/// Extension trait for converting standard Results to cucumber Results.
pub trait ResultExt<T> {
    /// Converts a Result to a CucumberError Result with context.
    fn with_cucumber_context(self, context: &str) -> Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_cucumber_context(self, context: &str) -> Result<T> {
        self.map_err(|e| {
            CucumberError::Config(ConfigError::InvalidCliArgs {
                reason: format!("{context}: {e}"),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_cucumber_error_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let cucumber_err = CucumberError::Io(io_err);
        assert!(cucumber_err.to_string().contains("I/O operation failed"));
    }

    #[test]
    fn test_step_panic_error() {
        let err = CucumberError::step_panic("test panic message");
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
        
        if let CucumberError::Step(StepError::Panic { message, payload: Some(p) }) = err {
            assert_eq!(message, "test panic");
            assert_eq!(p.to_readable_string(), "panic payload");
        } else {
            panic!("Expected step panic error with payload");
        }
    }

    #[test]
    fn test_no_step_match_error() {
        let err = CucumberError::no_step_match("Given I do something");
        assert!(err.to_string().contains("No matching step found for: Given I do something"));
    }

    #[test]
    fn test_ambiguous_step_error() {
        let err = CucumberError::ambiguous_step("Given I do something", 3);
        assert!(err.to_string().contains("Ambiguous step: Given I do something matches 3 step definitions"));
    }

    #[test]
    fn test_world_creation_error() {
        let source_err = io::Error::new(io::ErrorKind::Other, "world creation failed");
        let err = CucumberError::world_creation(source_err);
        assert!(err.to_string().contains("Failed to create World"));
    }

    #[test]
    fn test_invalid_retry_config_error() {
        let err = CucumberError::invalid_retry_config("negative retry count");
        assert!(err.to_string().contains("Invalid retry configuration: negative retry count"));
    }

    #[test]
    fn test_step_error_variants() {
        let timeout_err = StepError::Timeout {
            step_text: "Given I wait".to_string(),
            duration: std::time::Duration::from_secs(30),
        };
        assert!(timeout_err.to_string().contains("Step timed out after"));
        assert!(timeout_err.to_string().contains("Given I wait"));

        let no_match_err = StepError::NoMatch {
            step_text: "Given unknown step".to_string(),
        };
        assert!(no_match_err.to_string().contains("No matching step found for: Given unknown step"));

        let ambiguous_err = StepError::Ambiguous {
            step_text: "Given ambiguous step".to_string(),
            count: 2,
        };
        assert!(ambiguous_err.to_string().contains("Ambiguous step: Given ambiguous step matches 2 step definitions"));
    }

    #[test]
    fn test_world_error_variants() {
        let creation_err = WorldError::Creation {
            source: Box::new(io::Error::new(io::ErrorKind::Other, "creation failed")),
        };
        assert!(creation_err.to_string().contains("Failed to create World"));

        let invalid_state_err = WorldError::InvalidState {
            reason: "missing required field".to_string(),
        };
        assert!(invalid_state_err.to_string().contains("World is in invalid state: missing required field"));
    }

    #[test]
    fn test_writer_error_variants() {
        let io_err = WriterError::Io(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed"));
        assert!(io_err.to_string().contains("I/O error"));

        let format_err = WriterError::Format(fmt::Error);
        assert!(format_err.to_string().contains("Format error"));

        let unavailable_err = WriterError::Unavailable {
            reason: "buffer full".to_string(),
        };
        assert!(unavailable_err.to_string().contains("Output unavailable: buffer full"));
    }

    #[test]
    fn test_config_error_variants() {
        let retry_err = ConfigError::InvalidRetry {
            reason: "negative count".to_string(),
        };
        assert!(retry_err.to_string().contains("Invalid retry configuration: negative count"));

        let tag_filter_err = ConfigError::InvalidTagFilter {
            expression: "@invalid &&& @bad".to_string(),
        };
        assert!(tag_filter_err.to_string().contains("Invalid tag filter: @invalid &&& @bad"));

        let file_not_found_err = ConfigError::FeatureFileNotFound {
            path: "/path/to/missing.feature".to_string(),
        };
        assert!(file_not_found_err.to_string().contains("Feature file not found: /path/to/missing.feature"));

        let cli_args_err = ConfigError::InvalidCliArgs {
            reason: "conflicting arguments".to_string(),
        };
        assert!(cli_args_err.to_string().contains("Invalid CLI arguments: conflicting arguments"));
    }

    #[test]
    fn test_panic_payload_ext() {
        let string_payload: Arc<dyn std::any::Any + Send + 'static> = Arc::new("string panic".to_string());
        assert_eq!(string_payload.to_readable_string(), "string panic");

        let str_payload: Arc<dyn std::any::Any + Send + 'static> = Arc::new("str panic");
        assert_eq!(str_payload.to_readable_string(), "str panic");

        let unknown_payload: Arc<dyn std::any::Any + Send + 'static> = Arc::new(42i32);
        assert_eq!(unknown_payload.to_readable_string(), "Unknown panic payload");
    }

    #[test]
    fn test_from_conversions() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let cucumber_err: CucumberError = io_err.into();
        assert!(matches!(cucumber_err, CucumberError::Io(_)));

        let writer_err = WriterError::Format(fmt::Error);
        let cucumber_err: CucumberError = writer_err.into();
        assert!(matches!(cucumber_err, CucumberError::Writer(_)));

        let fmt_err = fmt::Error;
        let writer_err: WriterError = fmt_err.into();
        assert!(matches!(writer_err, WriterError::Format(_)));
    }

    #[test]
    fn test_result_ext() {
        let ok_result: std::result::Result<i32, io::Error> = Ok(42);
        let cucumber_result = ok_result.with_cucumber_context("test context");
        assert!(cucumber_result.is_ok());
        assert_eq!(cucumber_result.unwrap(), 42);

        let err_result: std::result::Result<i32, io::Error> = Err(io::Error::new(io::ErrorKind::Other, "test error"));
        let cucumber_result = err_result.with_cucumber_context("test context");
        assert!(cucumber_result.is_err());
        let err = cucumber_result.unwrap_err();
        assert!(err.to_string().contains("test context"));
        assert!(err.to_string().contains("test error"));
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