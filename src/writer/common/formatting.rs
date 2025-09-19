// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Output formatting utilities for writers.

use std::io::{self, Write};

use crate::error::{WriterResult, WriterError};

/// Utility trait for common output formatting operations.
pub trait OutputFormatter {
    /// The output type (typically something that implements `io::Write`).
    type Output;

    /// Gets a mutable reference to the output.
    fn output_mut(&mut self) -> &mut Self::Output;

    /// Writes a line to the output with error handling.
    fn write_line(&mut self, line: &str) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        writeln!(self.output_mut(), "{line}").map_err(|e| WriterError::from(e))
    }

    /// Writes raw bytes to the output with error handling.
    fn write_bytes(&mut self, bytes: &[u8]) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        self.output_mut().write_all(bytes).map_err(|e| WriterError::from(e))
    }

    /// Writes a formatted string to the output.
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        write!(self.output_mut(), "{args}").map_err(|e| WriterError::from(e))
    }

    /// Flushes the output if supported.
    fn flush(&mut self) -> WriterResult<()>
    where
        Self::Output: io::Write,
    {
        self.output_mut().flush().map_err(|e| WriterError::from(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};

    // Mock writer for testing OutputFormatter
    struct MockWriter {
        buffer: Vec<u8>,
        should_fail: bool,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                buffer: Vec::new(),
                should_fail: false,
            }
        }

        fn with_failure() -> Self {
            Self {
                buffer: Vec::new(),
                should_fail: true,
            }
        }

        fn written_content(&self) -> String {
            String::from_utf8_lossy(&self.buffer).to_string()
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.should_fail {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock failure"))
            } else {
                self.buffer.extend_from_slice(buf);
                Ok(buf.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.should_fail {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "mock failure"))
            } else {
                Ok(())
            }
        }
    }

    impl OutputFormatter for MockWriter {
        type Output = Self;

        fn output_mut(&mut self) -> &mut Self::Output {
            self
        }
    }

    #[test]
    fn output_formatter_write_line_success() {
        let mut writer = MockWriter::new();
        
        writer.write_line("test line").expect("should write successfully");
        
        assert_eq!(writer.written_content(), "test line\n");
    }

    #[test]
    fn output_formatter_write_line_failure() {
        let mut writer = MockWriter::with_failure();
        
        let result = writer.write_line("test line");
        
        assert!(result.is_err());
        match result.unwrap_err() {
            WriterError::Io(_) => {}, // Expected
            _ => panic!("Expected IO error"),
        }
    }

    #[test]
    fn output_formatter_write_bytes_success() {
        let mut writer = MockWriter::new();
        
        writer.write_bytes(b"test bytes").expect("should write successfully");
        
        assert_eq!(writer.written_content(), "test bytes");
    }

    #[test]
    fn output_formatter_write_bytes_failure() {
        let mut writer = MockWriter::with_failure();
        
        let result = writer.write_bytes(b"test bytes");
        
        assert!(result.is_err());
    }

    #[test]
    fn output_formatter_write_fmt_success() {
        let mut writer = MockWriter::new();
        
        <MockWriter as OutputFormatter>::write_fmt(&mut writer, format_args!("test {} {}", "formatted", 123))
            .expect("should write successfully");
        
        assert_eq!(writer.written_content(), "test formatted 123");
    }

    #[test]
    fn output_formatter_flush_success() {
        let mut writer = MockWriter::new();
        
        <MockWriter as OutputFormatter>::flush(&mut writer).expect("should flush successfully");
    }

    #[test]
    fn output_formatter_flush_failure() {
        let mut writer = MockWriter::with_failure();
        
        let result = <MockWriter as OutputFormatter>::flush(&mut writer);
        
        assert!(result.is_err());
    }

    #[test]
    fn output_formatter_multiple_operations() {
        let mut writer = MockWriter::new();
        
        writer.write_line("Line 1").expect("should write successfully");
        writer.write_bytes(b"Raw bytes").expect("should write successfully");
        <MockWriter as OutputFormatter>::write_fmt(&mut writer, format_args!("Formatted: {}", 42)).expect("should write successfully");
        <MockWriter as OutputFormatter>::flush(&mut writer).expect("should flush successfully");
        
        let content = writer.written_content();
        assert!(content.contains("Line 1\n"));
        assert!(content.contains("Raw bytes"));
        assert!(content.contains("Formatted: 42"));
    }

    #[test]
    fn output_formatter_empty_line() {
        let mut writer = MockWriter::new();
        
        writer.write_line("").expect("should write successfully");
        
        assert_eq!(writer.written_content(), "\n");
    }

    #[test]
    fn output_formatter_empty_bytes() {
        let mut writer = MockWriter::new();
        
        writer.write_bytes(&[]).expect("should write successfully");
        
        assert_eq!(writer.written_content(), "");
    }

    #[test]
    fn output_formatter_write_fmt_empty() {
        let mut writer = MockWriter::new();
        
        <MockWriter as OutputFormatter>::write_fmt(&mut writer, format_args!(""))
            .expect("should write successfully");
        
        assert_eq!(writer.written_content(), "");
    }

    #[test]
    fn output_formatter_error_propagation() {
        let mut writer = MockWriter::with_failure();
        
        // All operations should fail and propagate errors
        assert!(writer.write_line("test").is_err());
        assert!(writer.write_bytes(b"test").is_err());
        assert!(<MockWriter as OutputFormatter>::write_fmt(&mut writer, format_args!("test")).is_err());
        assert!(<MockWriter as OutputFormatter>::flush(&mut writer).is_err());
    }
}