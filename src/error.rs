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