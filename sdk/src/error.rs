use std::fmt;

use cucumber::writer::messages::rerun;
use serde::Serialize;

/// Stable error classes for protocol version 1.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ErrorCode {
    InvalidRequest,
    UnsupportedVersion,
    UnknownOperation,
    SourceError,
    InvalidMessages,
    IncompleteRun,
    RunAlreadyFinished,
    MissingReference,
    InternalError,
}

/// Structured failure without invented source locations or replacement IDs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkError {
    pub code: ErrorCode,
    pub message: String,
    /// Original exact URI for every SOURCE_ERROR, including an empty URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Physical NDJSON error line, when applicable, as an exact decimal string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
}

impl SdkError {
    /// Creates an error without context; extensions may add their own context.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            uri: None,
            line: None,
            reference_kind: None,
            reference_id: None,
        }
    }
}

impl fmt::Display for SdkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for SdkError {}

impl From<rerun::Error> for SdkError {
    fn from(error: rerun::Error) -> Self {
        let code = match &error {
            rerun::Error::IncompleteRun => ErrorCode::IncompleteRun,
            rerun::Error::RunAlreadyFinished => ErrorCode::RunAlreadyFinished,
            rerun::Error::MissingReference { .. } => {
                ErrorCode::MissingReference
            }
            rerun::Error::InvalidEnvelope(_) => ErrorCode::InvalidMessages,
            _ => ErrorCode::InternalError,
        };
        let mut result = Self::new(code, error.to_string());
        if let rerun::Error::MissingReference { kind, id } = error {
            result.reference_kind = Some(kind.to_owned());
            result.reference_id = Some(id);
        }
        result
    }
}
