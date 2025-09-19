// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Specialized formatters for world and error output.

use std::fmt::Debug;

use crate::writer::Verbosity;

/// Helper for handling world output based on verbosity settings.
#[derive(Debug, Clone, Copy)]
pub struct WorldFormatter;

impl WorldFormatter {
    /// Formats world output if verbosity allows it.
    pub fn format_world_if_needed<W: Debug>(
        world: Option<&W>,
        verbosity: Verbosity,
    ) -> Option<String> {
        if verbosity.shows_world() {
            world.map(|w| format!("{w:#?}"))
        } else {
            None
        }
    }

    /// Formats docstring output if verbosity allows it.
    pub fn format_docstring_if_needed(
        step: &gherkin::Step,
        verbosity: Verbosity,
    ) -> Option<&str> {
        if verbosity.shows_docstring() {
            step.docstring.as_deref()
        } else {
            None
        }
    }
}

/// Helper for common error message formatting.
#[derive(Debug, Clone, Copy)]
pub struct ErrorFormatter;

impl ErrorFormatter {
    /// Formats an error message with context.
    pub fn format_with_context(error: &dyn std::error::Error, context: &str) -> String {
        format!("{context}: {error}")
    }

    /// Formats a panic message from step execution.
    pub fn format_panic_message(info: &crate::event::Info) -> String {
        if let Some(msg) = info.downcast_ref::<String>() {
            msg.clone()
        } else if let Some(&msg) = info.downcast_ref::<&str>() {
            msg.to_string()
        } else {
            "Unknown error".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::Verbosity;

    #[test]
    fn world_formatter_respects_verbosity_default() {
        let world = Some(&42);
        
        let result = WorldFormatter::format_world_if_needed(world, Verbosity::Default);
        assert!(result.is_none());
    }

    #[test]
    fn world_formatter_respects_verbosity_show_world() {
        let world = Some(&42);
        
        let result = WorldFormatter::format_world_if_needed(world, Verbosity::ShowWorld);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "42");
    }

    #[test]
    fn world_formatter_respects_verbosity_show_world_and_docstring() {
        let world = Some(&"test_world");
        
        let result = WorldFormatter::format_world_if_needed(world, Verbosity::ShowWorldAndDocString);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "\"test_world\"");
    }

    #[test]
    fn world_formatter_handles_none_world() {
        let world: Option<&i32> = None;
        
        let result = WorldFormatter::format_world_if_needed(world, Verbosity::ShowWorld);
        assert!(result.is_none());
    }

    #[test]
    fn world_formatter_docstring_verbosity_default() {
        // Test docstring handling concept
        let shows_docstring = Verbosity::Default.shows_docstring();
        assert!(!shows_docstring);
    }

    #[test]
    fn world_formatter_docstring_verbosity_show_world() {
        // Test docstring handling concept  
        let shows_docstring = Verbosity::ShowWorld.shows_docstring();
        assert!(!shows_docstring);
    }

    #[test]
    fn world_formatter_docstring_verbosity_show_world_and_docstring() {
        // Test docstring handling concept
        let shows_docstring = Verbosity::ShowWorldAndDocString.shows_docstring();
        assert!(shows_docstring);
    }

    // Helper struct for testing docstring functionality
    struct MockStep {
        docstring: Option<String>,
    }

    impl MockStep {
        fn new(docstring: Option<&str>) -> Self {
            Self {
                docstring: docstring.map(String::from),
            }
        }
    }

    impl MockStep {
        fn docstring(&self) -> Option<&str> {
            self.docstring.as_deref()
        }
    }

    // We need to adjust the function signature for testing
    impl WorldFormatter {
        fn format_docstring_if_needed_mock(
            step: &MockStep,
            verbosity: Verbosity,
        ) -> Option<&str> {
            if verbosity.shows_docstring() {
                step.docstring()
            } else {
                None
            }
        }
    }

    #[test]
    fn world_formatter_docstring_mock_test() {
        let step_with_docstring = MockStep::new(Some("test docstring"));
        let step_without_docstring = MockStep::new(None);
        
        // Test with docstring showing verbosity
        assert_eq!(
            WorldFormatter::format_docstring_if_needed_mock(&step_with_docstring, Verbosity::ShowWorldAndDocString),
            Some("test docstring")
        );
        
        // Test without docstring showing verbosity
        assert_eq!(
            WorldFormatter::format_docstring_if_needed_mock(&step_with_docstring, Verbosity::ShowWorld),
            None
        );
        
        // Test with None docstring
        assert_eq!(
            WorldFormatter::format_docstring_if_needed_mock(&step_without_docstring, Verbosity::ShowWorldAndDocString),
            None
        );
    }

    #[test]
    fn error_formatter_formats_with_context() {
        let error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        
        let formatted = ErrorFormatter::format_with_context(&error, "File operation");
        
        assert_eq!(formatted, "File operation: file not found");
    }

    #[test]
    fn error_formatter_formats_panic_message_string() {
        let panic_msg = "test panic message".to_string();
        let info: crate::event::Info = std::sync::Arc::new(panic_msg);
        
        let formatted = ErrorFormatter::format_panic_message(&info);
        
        assert_eq!(formatted, "test panic message");
    }

    #[test]
    fn error_formatter_formats_panic_message_str() {
        let panic_msg = "test panic message";
        let info: crate::event::Info = std::sync::Arc::new(panic_msg);
        
        let formatted = ErrorFormatter::format_panic_message(&info);
        
        assert_eq!(formatted, "test panic message");
    }

    #[test]
    fn error_formatter_formats_panic_message_unknown() {
        let unknown_data = 42i32;
        let info: crate::event::Info = std::sync::Arc::new(unknown_data);
        
        let formatted = ErrorFormatter::format_panic_message(&info);
        
        assert_eq!(formatted, "Unknown error");
    }

    #[test]
    fn world_formatter_complex_types() {
        #[derive(Debug)]
        struct ComplexWorld {
            id: usize,
            name: String,
            values: Vec<i32>,
        }
        
        let world = ComplexWorld {
            id: 1,
            name: "test".to_string(),
            values: vec![1, 2, 3],
        };
        
        let result = WorldFormatter::format_world_if_needed(Some(&world), Verbosity::ShowWorld);
        assert!(result.is_some());
        
        let formatted = result.unwrap();
        assert!(formatted.contains("ComplexWorld"));
        assert!(formatted.contains("id: 1"));
        assert!(formatted.contains("name: \"test\""));
        assert!(formatted.contains("values"));
    }

    #[test]
    fn error_formatter_context_with_special_characters() {
        let error = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad input: {}[]");
        
        let formatted = ErrorFormatter::format_with_context(&error, "Operation with: special chars");
        
        assert_eq!(formatted, "Operation with: special chars: bad input: {}[]");
    }

    #[test]
    fn world_formatter_empty_string_world() {
        let world = Some(&"");
        
        let result = WorldFormatter::format_world_if_needed(world, Verbosity::ShowWorld);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "\"\"");
    }
}