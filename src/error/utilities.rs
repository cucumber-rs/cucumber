// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Utility traits and extensions for error handling.
//!
//! This module provides shared utilities for error conversion,
//! result extensions, and panic payload handling.

use super::{ConfigError, CucumberError};

/// Extension trait for converting standard Results to cucumber Results.
pub trait ResultExt<T> {
    /// Converts a Result to a CucumberError Result with context.
    fn with_cucumber_context(self, context: &str) -> crate::error::Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_cucumber_context(self, context: &str) -> crate::error::Result<T> {
        self.map_err(|e| {
            CucumberError::Config(ConfigError::InvalidCliArgs {
                reason: format!("{context}: {e}"),
            })
        })
    }
}

/// Extension trait for converting Results to specific error types with context.
pub trait ResultConfigExt<T> {
    /// Converts a Result to a ConfigError Result with context.
    fn with_config_context(self, context: &str) -> super::ConfigResult<T>;
}

impl<T, E> ResultConfigExt<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_config_context(self, context: &str) -> super::ConfigResult<T> {
        self.map_err(|e| ConfigError::InvalidCliArgs {
            reason: format!("{context}: {e}"),
        })
    }
}

/// Utility functions for error conversion and formatting.
pub mod conversion {
    use super::*;

    /// Converts an I/O error to a CucumberError with additional context.
    pub fn io_error_with_context(
        error: std::io::Error,
        context: &str,
    ) -> CucumberError {
        CucumberError::Config(ConfigError::InvalidCliArgs {
            reason: format!("{context}: {error}"),
        })
    }

    /// Converts any error to a CucumberError with context.
    pub fn any_error_with_context<E>(
        error: E,
        context: &str,
    ) -> CucumberError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        CucumberError::Config(ConfigError::InvalidCliArgs {
            reason: format!("{context}: {error}"),
        })
    }

    /// Creates a feature file not found error with path normalization.
    pub fn feature_file_not_found(path: &std::path::Path) -> CucumberError {
        CucumberError::Config(ConfigError::FeatureFileNotFound {
            path: path.display().to_string(),
        })
    }
}

/// Utility functions for error formatting and display.
pub mod formatting {
    /// Formats an error chain into a single string with context.
    pub fn format_error_chain(error: &dyn std::error::Error) -> String {
        let mut chain = Vec::new();
        let mut current = Some(error);

        while let Some(err) = current {
            chain.push(err.to_string());
            current = err.source();
        }

        if chain.len() == 1 {
            chain.into_iter().next().unwrap()
        } else {
            format!("{} (caused by: {})", chain[0], chain[1..].join(" -> "))
        }
    }

    /// Truncates error messages to a maximum length for display.
    pub fn truncate_error_message(message: &str, max_length: usize) -> String {
        if message.len() <= max_length {
            message.to_string()
        } else {
            format!("{}...", &message[..max_length.saturating_sub(3)])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[derive(Debug)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Test error: {}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn test_result_ext() {
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
    fn test_result_config_ext() {
        let ok_result: std::result::Result<String, TestError> = Ok("success".to_string());
        let config_result = ok_result.with_config_context("config parsing");
        assert!(config_result.is_ok());
        assert_eq!(config_result.unwrap(), "success");

        let err_result: std::result::Result<String, TestError> = 
            Err(TestError("parsing failed".to_string()));
        let config_result = err_result.with_config_context("config parsing");
        assert!(config_result.is_err());
        let err = config_result.unwrap_err();
        assert!(err.to_string().contains("config parsing"));
        assert!(err.to_string().contains("Test error: parsing failed"));
    }

    #[test]
    fn test_conversion_io_error_with_context() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let cucumber_err = conversion::io_error_with_context(io_err, "reading config file");
        
        assert!(cucumber_err.to_string().contains("reading config file"));
        assert!(cucumber_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_conversion_any_error_with_context() {
        let test_err = TestError("custom error".to_string());
        let cucumber_err = conversion::any_error_with_context(test_err, "processing data");
        
        assert!(cucumber_err.to_string().contains("processing data"));
        assert!(cucumber_err.to_string().contains("Test error: custom error"));
    }

    #[test]
    fn test_conversion_feature_file_not_found() {
        let path = std::path::Path::new("/path/to/missing.feature");
        let cucumber_err = conversion::feature_file_not_found(path);
        
        assert!(cucumber_err.to_string().contains("Feature file not found"));
        assert!(cucumber_err.to_string().contains("/path/to/missing.feature"));
    }

    #[test]
    fn test_formatting_error_chain() {
        let io_err = io::Error::new(io::ErrorKind::Other, "root cause");
        let simple_chain = formatting::format_error_chain(&io_err);
        assert_eq!(simple_chain, "root cause");

        // Test with a more complex error chain by creating a nested error
        let nested_err = TestError("wrapper error".to_string());
        let chain = formatting::format_error_chain(&nested_err);
        assert!(chain.contains("Test error: wrapper error"));
    }

    #[test]
    fn test_formatting_truncate_error_message() {
        let short_message = "short";
        assert_eq!(formatting::truncate_error_message(short_message, 10), "short");

        let long_message = "this is a very long error message that should be truncated";
        let truncated = formatting::truncate_error_message(long_message, 20);
        assert_eq!(truncated, "this is a very lo...");
        assert!(truncated.len() <= 20);

        let exact_length_message = "exactly twenty chars";
        assert_eq!(
            formatting::truncate_error_message(exact_length_message, 20),
            "exactly twenty chars"
        );
    }

    #[test]
    fn test_formatting_truncate_edge_cases() {
        let message = "test";
        assert_eq!(formatting::truncate_error_message(message, 0), "");
        assert_eq!(formatting::truncate_error_message(message, 1), "");
        assert_eq!(formatting::truncate_error_message(message, 2), "");
        assert_eq!(formatting::truncate_error_message(message, 3), "");
        assert_eq!(formatting::truncate_error_message(message, 4), "test");
    }

    #[test]
    fn test_multiple_extensions_usage() {
        let result: std::result::Result<i32, TestError> = Err(TestError("test".to_string()));
        
        // Test chaining different context methods
        let cucumber_result = result.with_cucumber_context("cucumber context");
        assert!(cucumber_result.is_err());
        
        let result2: std::result::Result<i32, TestError> = Err(TestError("test2".to_string()));
        let config_result = result2.with_config_context("config context");
        assert!(config_result.is_err());
    }
}