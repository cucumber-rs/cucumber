// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Step execution error types and utilities.
//!
//! This module defines errors that can occur during step execution,
//! including panics, matching failures, timeouts, and ambiguous matches.

use std::sync::Arc;
use std::time::Duration;

use derive_more::with_trait::{Display, Error};

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
        duration: Duration,
    },
}

/// Result type alias for step operations.
pub type StepResult<T> = std::result::Result<T, StepError>;

impl StepError {
    /// Creates a new panic error.
    #[must_use]
    pub fn panic(message: impl Into<String>) -> Self {
        Self::Panic {
            message: message.into(),
            payload: None,
        }
    }

    /// Creates a new panic error with payload.
    #[must_use]
    pub fn panic_with_payload(
        message: impl Into<String>,
        payload: Arc<dyn std::any::Any + Send + 'static>,
    ) -> Self {
        Self::Panic {
            message: message.into(),
            payload: Some(payload),
        }
    }

    /// Creates a new no match error.
    #[must_use]
    pub fn no_match(step_text: impl Into<String>) -> Self {
        Self::NoMatch {
            step_text: step_text.into(),
        }
    }

    /// Creates a new ambiguous error.
    #[must_use]
    pub fn ambiguous(step_text: impl Into<String>, count: usize) -> Self {
        Self::Ambiguous {
            step_text: step_text.into(),
            count,
        }
    }

    /// Creates a new timeout error.
    #[must_use]
    pub fn timeout(step_text: impl Into<String>, duration: Duration) -> Self {
        Self::Timeout {
            step_text: step_text.into(),
            duration,
        }
    }

    /// Returns the step text if available.
    #[must_use]
    pub fn step_text(&self) -> Option<&str> {
        match self {
            Self::Panic { .. } => None,
            Self::NoMatch { step_text } => Some(step_text),
            Self::Ambiguous { step_text, .. } => Some(step_text),
            Self::Timeout { step_text, .. } => Some(step_text),
        }
    }

    /// Returns true if this is a panic error.
    #[must_use]
    pub fn is_panic(&self) -> bool {
        matches!(self, Self::Panic { .. })
    }

    /// Returns true if this is a timeout error.
    #[must_use]
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_step_error_constructors() {
        let panic_err = StepError::panic("test panic");
        assert!(panic_err.is_panic());
        assert!(panic_err.to_string().contains("Step panicked: test panic"));

        let no_match_err = StepError::no_match("Given I do something");
        assert!(!no_match_err.is_panic());
        assert_eq!(no_match_err.step_text(), Some("Given I do something"));

        let ambiguous_err = StepError::ambiguous("Given ambiguous", 3);
        assert_eq!(ambiguous_err.step_text(), Some("Given ambiguous"));
        assert!(ambiguous_err.to_string().contains("matches 3 step definitions"));

        let timeout_err = StepError::timeout("Given I wait", Duration::from_secs(30));
        assert!(timeout_err.is_timeout());
        assert_eq!(timeout_err.step_text(), Some("Given I wait"));
    }

    #[test]
    fn test_step_error_with_payload() {
        let payload: Arc<dyn std::any::Any + Send + 'static> = Arc::new("payload".to_string());
        let err = StepError::panic_with_payload("panic message", payload.clone());
        
        if let StepError::Panic { message, payload: Some(p) } = err {
            assert_eq!(message, "panic message");
            assert_eq!(p.to_readable_string(), "payload");
        } else {
            panic!("Expected panic error with payload");
        }
    }

    #[test]
    fn test_step_text_extraction() {
        let panic_err = StepError::panic("test");
        assert_eq!(panic_err.step_text(), None);

        let no_match_err = StepError::no_match("Given step");
        assert_eq!(no_match_err.step_text(), Some("Given step"));

        let ambiguous_err = StepError::ambiguous("When step", 2);
        assert_eq!(ambiguous_err.step_text(), Some("When step"));

        let timeout_err = StepError::timeout("Then step", Duration::from_millis(500));
        assert_eq!(timeout_err.step_text(), Some("Then step"));
    }

    #[test]
    fn test_error_type_checks() {
        let panic_err = StepError::panic("test");
        assert!(panic_err.is_panic());
        assert!(!panic_err.is_timeout());

        let timeout_err = StepError::timeout("test", Duration::from_secs(1));
        assert!(!timeout_err.is_panic());
        assert!(timeout_err.is_timeout());

        let no_match_err = StepError::no_match("test");
        assert!(!no_match_err.is_panic());
        assert!(!no_match_err.is_timeout());
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
    fn test_step_error_variants_display() {
        let timeout_err = StepError::Timeout {
            step_text: "Given I wait".to_string(),
            duration: Duration::from_secs(30),
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
    fn test_step_result_type() {
        let ok_result: StepResult<i32> = Ok(42);
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: StepResult<i32> = Err(StepError::panic("test"));
        assert!(err_result.is_err());
        assert!(err_result.unwrap_err().is_panic());
    }
}