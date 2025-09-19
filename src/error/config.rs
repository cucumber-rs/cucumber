// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Configuration and validation error types.
//!
//! This module defines errors that can occur during configuration parsing,
//! validation, and CLI argument processing.

use derive_more::with_trait::{Display, Error};

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

/// Result type alias for configuration operations.
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

impl ConfigError {
    /// Creates a new invalid retry error.
    #[must_use]
    pub fn invalid_retry(reason: impl Into<String>) -> Self {
        Self::InvalidRetry {
            reason: reason.into(),
        }
    }

    /// Creates a new invalid tag filter error.
    #[must_use]
    pub fn invalid_tag_filter(expression: impl Into<String>) -> Self {
        Self::InvalidTagFilter {
            expression: expression.into(),
        }
    }

    /// Creates a new feature file not found error.
    #[must_use]
    pub fn feature_file_not_found(path: impl Into<String>) -> Self {
        Self::FeatureFileNotFound {
            path: path.into(),
        }
    }

    /// Creates a new invalid CLI arguments error.
    #[must_use]
    pub fn invalid_cli_args(reason: impl Into<String>) -> Self {
        Self::InvalidCliArgs {
            reason: reason.into(),
        }
    }

    /// Returns true if this is an invalid retry error.
    #[must_use]
    pub fn is_invalid_retry(&self) -> bool {
        matches!(self, Self::InvalidRetry { .. })
    }

    /// Returns true if this is an invalid tag filter error.
    #[must_use]
    pub fn is_invalid_tag_filter(&self) -> bool {
        matches!(self, Self::InvalidTagFilter { .. })
    }

    /// Returns true if this is a feature file not found error.
    #[must_use]
    pub fn is_feature_file_not_found(&self) -> bool {
        matches!(self, Self::FeatureFileNotFound { .. })
    }

    /// Returns true if this is an invalid CLI arguments error.
    #[must_use]
    pub fn is_invalid_cli_args(&self) -> bool {
        matches!(self, Self::InvalidCliArgs { .. })
    }

    /// Returns the reason for invalid retry configuration, if applicable.
    #[must_use]
    pub fn invalid_retry_reason(&self) -> Option<&str> {
        match self {
            Self::InvalidRetry { reason } => Some(reason),
            _ => None,
        }
    }

    /// Returns the invalid tag filter expression, if applicable.
    #[must_use]
    pub fn invalid_tag_filter_expression(&self) -> Option<&str> {
        match self {
            Self::InvalidTagFilter { expression } => Some(expression),
            _ => None,
        }
    }

    /// Returns the missing feature file path, if applicable.
    #[must_use]
    pub fn missing_feature_file_path(&self) -> Option<&str> {
        match self {
            Self::FeatureFileNotFound { path } => Some(path),
            _ => None,
        }
    }

    /// Returns the reason for invalid CLI arguments, if applicable.
    #[must_use]
    pub fn invalid_cli_args_reason(&self) -> Option<&str> {
        match self {
            Self::InvalidCliArgs { reason } => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_constructors() {
        let retry_err = ConfigError::invalid_retry("negative count");
        assert!(retry_err.is_invalid_retry());
        assert_eq!(retry_err.invalid_retry_reason(), Some("negative count"));
        assert!(retry_err.to_string().contains("Invalid retry configuration: negative count"));

        let tag_filter_err = ConfigError::invalid_tag_filter("@invalid &&& @bad");
        assert!(tag_filter_err.is_invalid_tag_filter());
        assert_eq!(tag_filter_err.invalid_tag_filter_expression(), Some("@invalid &&& @bad"));
        assert!(tag_filter_err.to_string().contains("Invalid tag filter: @invalid &&& @bad"));

        let file_not_found_err = ConfigError::feature_file_not_found("/path/to/missing.feature");
        assert!(file_not_found_err.is_feature_file_not_found());
        assert_eq!(file_not_found_err.missing_feature_file_path(), Some("/path/to/missing.feature"));
        assert!(file_not_found_err.to_string().contains("Feature file not found: /path/to/missing.feature"));

        let cli_args_err = ConfigError::invalid_cli_args("conflicting options");
        assert!(cli_args_err.is_invalid_cli_args());
        assert_eq!(cli_args_err.invalid_cli_args_reason(), Some("conflicting options"));
        assert!(cli_args_err.to_string().contains("Invalid CLI arguments: conflicting options"));
    }

    #[test]
    fn test_config_error_type_checks() {
        let retry_err = ConfigError::invalid_retry("test");
        assert!(retry_err.is_invalid_retry());
        assert!(!retry_err.is_invalid_tag_filter());
        assert!(!retry_err.is_feature_file_not_found());
        assert!(!retry_err.is_invalid_cli_args());

        let tag_filter_err = ConfigError::invalid_tag_filter("test");
        assert!(!tag_filter_err.is_invalid_retry());
        assert!(tag_filter_err.is_invalid_tag_filter());
        assert!(!tag_filter_err.is_feature_file_not_found());
        assert!(!tag_filter_err.is_invalid_cli_args());

        let file_not_found_err = ConfigError::feature_file_not_found("test.feature");
        assert!(!file_not_found_err.is_invalid_retry());
        assert!(!file_not_found_err.is_invalid_tag_filter());
        assert!(file_not_found_err.is_feature_file_not_found());
        assert!(!file_not_found_err.is_invalid_cli_args());

        let cli_args_err = ConfigError::invalid_cli_args("test");
        assert!(!cli_args_err.is_invalid_retry());
        assert!(!cli_args_err.is_invalid_tag_filter());
        assert!(!cli_args_err.is_feature_file_not_found());
        assert!(cli_args_err.is_invalid_cli_args());
    }

    #[test]
    fn test_config_error_value_extraction() {
        let retry_err = ConfigError::invalid_retry("negative value");
        assert_eq!(retry_err.invalid_retry_reason(), Some("negative value"));
        assert_eq!(retry_err.invalid_tag_filter_expression(), None);
        assert_eq!(retry_err.missing_feature_file_path(), None);
        assert_eq!(retry_err.invalid_cli_args_reason(), None);

        let tag_filter_err = ConfigError::invalid_tag_filter("@bad-syntax");
        assert_eq!(tag_filter_err.invalid_retry_reason(), None);
        assert_eq!(tag_filter_err.invalid_tag_filter_expression(), Some("@bad-syntax"));
        assert_eq!(tag_filter_err.missing_feature_file_path(), None);
        assert_eq!(tag_filter_err.invalid_cli_args_reason(), None);

        let file_not_found_err = ConfigError::feature_file_not_found("missing.feature");
        assert_eq!(file_not_found_err.invalid_retry_reason(), None);
        assert_eq!(file_not_found_err.invalid_tag_filter_expression(), None);
        assert_eq!(file_not_found_err.missing_feature_file_path(), Some("missing.feature"));
        assert_eq!(file_not_found_err.invalid_cli_args_reason(), None);

        let cli_args_err = ConfigError::invalid_cli_args("incompatible flags");
        assert_eq!(cli_args_err.invalid_retry_reason(), None);
        assert_eq!(cli_args_err.invalid_tag_filter_expression(), None);
        assert_eq!(cli_args_err.missing_feature_file_path(), None);
        assert_eq!(cli_args_err.invalid_cli_args_reason(), Some("incompatible flags"));
    }

    #[test]
    fn test_config_error_display() {
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
    fn test_config_result_type() {
        let ok_result: ConfigResult<String> = Ok("success".to_string());
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), "success");

        let err_result: ConfigResult<String> = Err(ConfigError::invalid_retry("test"));
        assert!(err_result.is_err());
        assert!(err_result.unwrap_err().is_invalid_retry());
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
}