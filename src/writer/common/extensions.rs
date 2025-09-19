// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Extension traits for writer operations.

/// Extension methods for common writer operations.
pub trait WriterExt {
    /// Handles write errors gracefully by logging warnings instead of panicking.
    fn handle_write_error(self, context: &str);
}

impl<T, E: std::fmt::Display> WriterExt for Result<T, E> {
    fn handle_write_error(self, context: &str) {
        if let Err(e) = self {
            eprintln!("Warning: {context}: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writer_ext_handles_success_silently() {
        // Capture stderr to test the warning output
        let result: Result<(), &str> = Ok(());
        
        // This should not produce any output
        result.handle_write_error("Test context");
        // Test passes if no panic occurs
    }

    #[test] 
    fn writer_ext_handles_error_with_warning() {
        // This test verifies the concept - in practice we'd need to capture stderr
        let result: Result<(), &str> = Err("test error");
        
        // This should print a warning to stderr
        result.handle_write_error("Test context");
        // Test passes if no panic occurs and warning is printed
    }

    #[test]
    fn writer_ext_handles_different_error_types() {
        // Test with different error types
        let io_error: Result<(), std::io::Error> = Err(
            std::io::Error::new(std::io::ErrorKind::NotFound, "file not found")
        );
        io_error.handle_write_error("IO operation");
        
        let string_error: Result<(), String> = Err("custom error".to_string());
        string_error.handle_write_error("String operation");
        
        let static_str_error: Result<(), &'static str> = Err("static error");
        static_str_error.handle_write_error("Static operation");
    }

    #[test]
    fn writer_ext_handles_success_with_value() {
        let result: Result<i32, &str> = Ok(42);
        
        // Should handle success with a value silently
        result.handle_write_error("Test context with value");
        // Test passes if no panic occurs
    }

    #[test]
    fn writer_ext_handles_formatted_errors() {
        use std::fmt;
        
        #[derive(Debug)]
        struct CustomError {
            code: i32,
            message: String,
        }
        
        impl fmt::Display for CustomError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "Error {}: {}", self.code, self.message)
            }
        }
        
        impl std::error::Error for CustomError {}
        
        let custom_error = CustomError {
            code: 404,
            message: "Resource not found".to_string(),
        };
        
        let result: Result<(), CustomError> = Err(custom_error);
        result.handle_write_error("Custom error operation");
        // Should print "Warning: Custom error operation: Error 404: Resource not found"
    }

    #[test]
    fn writer_ext_context_formatting() {
        // Test that context is properly formatted in the warning
        let result: Result<(), &str> = Err("test error");
        
        // Test different context formats
        result.handle_write_error("Operation");
        
        let result2: Result<(), &str> = Err("another error");
        result2.handle_write_error("Long operation context with details");
        
        let result3: Result<(), &str> = Err("error with symbols: {}[]");
        result3.handle_write_error("Context with: special chars");
    }

    #[test]
    fn writer_ext_empty_context() {
        let result: Result<(), &str> = Err("test error");
        
        // Test with empty context
        result.handle_write_error("");
        // Should print "Warning: : test error"
    }

    #[test]
    fn writer_ext_chaining() {
        // Test that the extension can be used in method chaining scenarios
        fn operation_that_can_fail() -> Result<String, &'static str> {
            Err("operation failed")
        }
        
        operation_that_can_fail().handle_write_error("Chained operation");
        // Test passes if no panic occurs
    }
}