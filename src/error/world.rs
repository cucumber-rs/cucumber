// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! World trait implementation error types.
//!
//! This module defines errors that can occur during World creation and management,
//! including initialization failures and invalid state errors.

use derive_more::with_trait::{Display, Error};

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

/// Result type alias for world operations.
pub type WorldResult<T> = std::result::Result<T, WorldError>;

impl WorldError {
    /// Creates a new creation error.
    #[must_use]
    pub fn creation(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Creation {
            source: Box::new(source),
        }
    }

    /// Creates a new invalid state error.
    #[must_use]
    pub fn invalid_state(reason: impl Into<String>) -> Self {
        Self::InvalidState {
            reason: reason.into(),
        }
    }

    /// Returns the reason for invalid state, if applicable.
    #[must_use]
    pub fn invalid_state_reason(&self) -> Option<&str> {
        match self {
            Self::InvalidState { reason } => Some(reason),
            Self::Creation { .. } => None,
        }
    }

    /// Returns true if this is a creation error.
    #[must_use]
    pub fn is_creation_error(&self) -> bool {
        matches!(self, Self::Creation { .. })
    }

    /// Returns true if this is an invalid state error.
    #[must_use]
    pub fn is_invalid_state(&self) -> bool {
        matches!(self, Self::InvalidState { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[derive(Debug)]
    struct CustomError(String);

    impl std::fmt::Display for CustomError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Custom error: {}", self.0)
        }
    }

    impl std::error::Error for CustomError {}

    #[test]
    fn test_world_error_constructors() {
        let creation_err = WorldError::creation(io::Error::new(io::ErrorKind::Other, "creation failed"));
        assert!(creation_err.is_creation_error());
        assert!(!creation_err.is_invalid_state());
        assert!(creation_err.to_string().contains("Failed to create World"));

        let invalid_state_err = WorldError::invalid_state("missing required field");
        assert!(!invalid_state_err.is_creation_error());
        assert!(invalid_state_err.is_invalid_state());
        assert_eq!(invalid_state_err.invalid_state_reason(), Some("missing required field"));
    }

    #[test]
    fn test_world_error_display() {
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
    fn test_invalid_state_reason_extraction() {
        let creation_err = WorldError::creation(CustomError("test".to_string()));
        assert_eq!(creation_err.invalid_state_reason(), None);

        let invalid_state_err = WorldError::invalid_state("test reason");
        assert_eq!(invalid_state_err.invalid_state_reason(), Some("test reason"));
    }

    #[test]
    fn test_error_type_checks() {
        let creation_err = WorldError::creation(io::Error::new(io::ErrorKind::Other, "test"));
        assert!(creation_err.is_creation_error());
        assert!(!creation_err.is_invalid_state());

        let invalid_state_err = WorldError::invalid_state("test");
        assert!(!invalid_state_err.is_creation_error());
        assert!(invalid_state_err.is_invalid_state());
    }

    #[test]
    fn test_world_result_type() {
        let ok_result: WorldResult<i32> = Ok(42);
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: WorldResult<i32> = Err(WorldError::invalid_state("test"));
        assert!(err_result.is_err());
        assert!(err_result.unwrap_err().is_invalid_state());
    }

    #[test]
    fn test_custom_error_source() {
        let custom_err = CustomError("custom message".to_string());
        let world_err = WorldError::creation(custom_err);
        
        assert!(world_err.source().is_some());
        if let Some(source) = world_err.source() {
            assert!(source.to_string().contains("Custom error: custom message"));
        }
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
    fn test_error_chain() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let world_err = WorldError::creation(io_err);
        
        assert!(world_err.source().is_some());
        if let Some(source) = world_err.source() {
            assert!(source.to_string().contains("access denied"));
        }
    }
}