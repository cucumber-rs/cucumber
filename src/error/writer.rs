// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Writer-specific error types and utilities.
//!
//! This module defines errors that can occur during output writing operations,
//! including I/O errors, serialization failures, formatting errors, and output unavailability.

use std::{fmt, io};

use derive_more::with_trait::{Display, Error};

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

/// Result type alias for writer operations.
pub type WriterResult<T> = std::result::Result<T, WriterError>;

impl WriterError {
    /// Creates a new unavailable error.
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::Unavailable {
            reason: reason.into(),
        }
    }

    /// Creates a new XML error.
    #[cfg(feature = "output-junit")]
    #[must_use]
    pub fn xml(message: impl Into<String>) -> Self {
        Self::Xml(message.into())
    }

    /// Returns true if this is an I/O error.
    #[must_use]
    pub fn is_io_error(&self) -> bool {
        matches!(self, Self::Io(_))
    }

    /// Returns true if this is a format error.
    #[must_use]
    pub fn is_format_error(&self) -> bool {
        matches!(self, Self::Format(_))
    }

    /// Returns true if this is an unavailable error.
    #[must_use]
    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }

    /// Returns the unavailable reason if applicable.
    #[must_use]
    pub fn unavailable_reason(&self) -> Option<&str> {
        match self {
            Self::Unavailable { reason } => Some(reason),
            _ => None,
        }
    }

    /// Returns true if this is a serialization error.
    #[cfg(any(feature = "output-json", feature = "libtest"))]
    #[must_use]
    pub fn is_serialization_error(&self) -> bool {
        matches!(self, Self::Serialization(_))
    }

    /// Returns true if this is an XML error.
    #[cfg(feature = "output-junit")]
    #[must_use]
    pub fn is_xml_error(&self) -> bool {
        matches!(self, Self::Xml(_))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_error_constructors() {
        let unavailable_err = WriterError::unavailable("buffer full");
        assert!(unavailable_err.is_unavailable());
        assert_eq!(unavailable_err.unavailable_reason(), Some("buffer full"));
        assert!(unavailable_err.to_string().contains("Output unavailable: buffer full"));

        #[cfg(feature = "output-junit")]
        {
            let xml_err = WriterError::xml("invalid XML structure");
            assert!(xml_err.is_xml_error());
            assert!(xml_err.to_string().contains("XML generation failed: invalid XML structure"));
        }
    }

    #[test]
    fn test_writer_error_type_checks() {
        let io_err = WriterError::Io(io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed"));
        assert!(io_err.is_io_error());
        assert!(!io_err.is_format_error());
        assert!(!io_err.is_unavailable());

        let format_err = WriterError::Format(fmt::Error);
        assert!(!format_err.is_io_error());
        assert!(format_err.is_format_error());
        assert!(!format_err.is_unavailable());

        let unavailable_err = WriterError::unavailable("test reason");
        assert!(!unavailable_err.is_io_error());
        assert!(!unavailable_err.is_format_error());
        assert!(unavailable_err.is_unavailable());
    }

    #[test]
    fn test_writer_error_display() {
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
    fn test_from_conversions() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let writer_err: WriterError = io_err.into();
        assert!(writer_err.is_io_error());

        let fmt_err = fmt::Error;
        let writer_err: WriterError = fmt_err.into();
        assert!(writer_err.is_format_error());
    }

    #[cfg(any(feature = "output-json", feature = "libtest"))]
    #[test]
    fn test_serialization_error() {
        use serde_json::json;
        
        // Create a serialization error by trying to serialize an invalid value
        let invalid_json = "\x00\x01\x02";
        let parse_err = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let writer_err: WriterError = parse_err.into();
        
        assert!(writer_err.is_serialization_error());
        assert!(writer_err.to_string().contains("Serialization failed"));
    }

    #[test]
    fn test_unavailable_reason_extraction() {
        let io_err = WriterError::Io(io::Error::new(io::ErrorKind::Other, "test"));
        assert_eq!(io_err.unavailable_reason(), None);

        let unavailable_err = WriterError::unavailable("output closed");
        assert_eq!(unavailable_err.unavailable_reason(), Some("output closed"));
    }

    #[test]
    fn test_writer_result_type() {
        let ok_result: WriterResult<String> = Ok("success".to_string());
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), "success");

        let err_result: WriterResult<String> = Err(WriterError::unavailable("test"));
        assert!(err_result.is_err());
        assert!(err_result.unwrap_err().is_unavailable());
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

        #[cfg(feature = "output-junit")]
        {
            let xml_err = WriterError::Xml("malformed XML".to_string());
            assert!(xml_err.to_string().contains("XML generation failed: malformed XML"));
        }
    }

    #[test]
    fn test_error_chain() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "write permission denied");
        let writer_err = WriterError::Io(io_err);
        
        assert!(writer_err.source().is_some());
        if let Some(source) = writer_err.source() {
            assert!(source.to_string().contains("write permission denied"));
        }
    }
}