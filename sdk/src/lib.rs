//! In-process source compilation, lossless rerun planning and Rust extensions.
//!
//! The versioned JSON interface delegates to the same typed operations as Rust
//! callers. It neither discovers files nor executes foreign step definitions.
//! Core graph fields are preserved, including absent optional upstream fields.
#![forbid(unsafe_code)]

mod error;
mod json_input;
mod model;

use std::{
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
};

use cucumber::writer::messages::{rerun, source::SourceGraph};
pub use error::{ErrorCode, SdkError};
pub use model::{
    Attempt, CompileInput, Compiled, Entry, RerunPlan, SourceFormat,
};
use serde_json::{Map, Value, json};

/// A Rust-owned operation, invoked separately from the versioned core protocol.
///
/// Names have at least two dot-separated ASCII identifier segments, for example
/// `verification.summary`. The `core` namespace is reserved. The registry takes
/// ownership of the implementation; no host-language callbacks are registered.
/// Extensions must maintain their own invariants if their code unwinds.
pub trait Extension {
    /// Returns the qualified name captured once at registration.
    fn name(&self) -> &str;
    /// Performs the application's operation on its application-defined payload.
    fn execute(&self, input: &Value) -> Result<Value, SdkError>;
}

/// Independent native SDK session with an instance-owned extension registry.
///
/// Compilation and rerun requests have no shared mutable run state. Ordinary
/// unwinding panics at public operation boundaries become `INTERNAL_ERROR`.
/// Abort, OOM and process faults cannot be recovered by this API.
#[derive(Default)]
pub struct Sdk {
    extensions: HashMap<String, Box<dyn Extension>>,
}

impl Sdk {
    /// Creates an empty SDK with the built-in compile and rerun operations.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles the adopted version 1 JSON request/response protocol.
    ///
    /// Error responses never contain a success result. Invalid requests do not
    /// alter the session. Native library loading occurs outside this boundary.
    #[must_use]
    pub fn request(&self, input: &str) -> String {
        match catch_unwind(AssertUnwindSafe(|| {
            let response = match self.dispatch(input) {
                Ok(result) => json!({
                "version": 1,
                 "ok": true,
                 "result": result}
                ),

                Err(error) => json!({
                "version": 1,
                 "ok": false,
                 "error": error}
                ),
            };
            response.to_string()
        })) {
            Ok(response) => response,
            Err(_) => "{\"version\":1,\"ok\":false,\"error\":{\"code\":\"INTE\
                RNAL_ERROR\",\"message\":\"native SDK operation panicked\"}}"
                .to_owned(),
        }
    }

    /// Reports implemented operations and pinned compatibility targets.
    ///
    /// Baselines are target identities, not a claim of complete schema or
    /// framework parity. Foreign step execution is explicitly not implemented.
    #[must_use]
    pub fn capabilities(&self) -> Value {
        json!({
            "kind": "capabilities", "protocolVersions": [1],
            "operations": ["capabilities", "compile", "rerun"],
            "formats": ["gherkin", "markdown"],
            "baselines": {"gherkin": "43.0.0", "cucumberJs": "13.3.0",
                "messages": ["34.2.1", "35.0.0"]},
            "foreignStepExecution": false
        })
    }

    /// Compiles supplied source without filesystem access or syntax guessing.
    ///
    /// # Errors
    /// Empty default language is invalid input. Source failures preserve the
    /// exact supplied URI, including the empty string. Native panics are
    /// caught.
    pub fn compile(
        &self,
        input: CompileInput<'_>,
    ) -> Result<Compiled, SdkError> {
        guarded(|| {
            if input.default_language.is_empty() {
                return Err(invalid("defaultLanguage must not be empty"));
            }
            let graph = match input.format {
                SourceFormat::Gherkin => SourceGraph::parse_gherkin43(
                    input.uri,
                    input.source,
                    input.default_language,
                ),
                SourceFormat::Markdown => {
                    SourceGraph::parse_markdown_gherkin43(
                        input.uri,
                        input.source,
                        input.default_language,
                    )
                }
            }
            .map_err(|error| {
                let mut result =
                    SdkError::new(ErrorCode::SourceError, error.to_string());
                result.uri = Some(input.uri.to_owned());
                result
            })?;
            Ok(Compiled {
                uri: input.uri.to_owned(),
                format: input.format,
                gherkin_document: graph.gherkin_document().clone(),
                pickles: graph.pickles().to_vec(),
            })
        })
    }

    /// Derives a rerun plan from original NDJSON using the core collector.
    ///
    /// # Errors
    /// Requires exactly one complete run and valid relevant envelopes. Error
    /// `line`, when present, identifies the physical NDJSON line, not a source
    /// line. Empty lines are ignored; every nonempty line must be a JSON
    /// object.
    pub fn rerun(
        &self,
        ndjson: &str,
        separator: &str,
    ) -> Result<RerunPlan, SdkError> {
        guarded(|| {
            let mut collector = rerun::Collector::new();
            for (index, text) in ndjson.lines().enumerate() {
                if text.trim().is_empty() {
                    continue;
                }
                let envelope = json_input::parse(text).map_err(|error| {
                    message_error(error.to_string(), index + 1)
                })?;
                if !envelope.is_object() {
                    return Err(message_error(
                        "NDJSON envelope must be an object",
                        index + 1,
                    ));
                }
                collector.ingest(&envelope).map_err(|error| {
                    let mut result = SdkError::from(error);
                    result.line = Some((index + 1).to_string());
                    result
                })?;
            }
            let plan = collector.plan().map_err(SdkError::from)?;
            let attempts = plan
                .attempts()
                .iter()
                .map(|attempt| {
                    let status = match attempt.status {
                        rerun::Status::Unknown => "UNKNOWN",
                        rerun::Status::Passed => "PASSED",
                        rerun::Status::Skipped => "SKIPPED",
                        rerun::Status::Pending => "PENDING",
                        rerun::Status::Undefined => "UNDEFINED",
                        rerun::Status::Ambiguous => "AMBIGUOUS",
                        rerun::Status::Failed => "FAILED",
                        _ => {
                            return Err(SdkError::new(
                                ErrorCode::InternalError,
                                "unsupported core attempt status",
                            ));
                        }
                    };
                    Ok(Attempt {
                        started_id: attempt.started_id.clone(),
                        test_case_id: attempt.test_case_id.clone(),
                        pickle_id: attempt.pickle_id.clone(),
                        attempt: attempt.attempt.to_string(),
                        uri: attempt.uri.clone(),
                        line: attempt.line.to_string(),
                        status: status.to_owned(),
                        completed: attempt.completed,
                        will_be_retried: attempt.will_be_retried,
                    })
                })
                .collect::<Result<Vec<_>, SdkError>>()?;
            let entries = plan
                .entries()
                .iter()
                .map(|entry| Entry {
                    uri: entry.uri.clone(),
                    lines: entry.lines.iter().map(usize::to_string).collect(),
                })
                .collect();
            Ok(RerunPlan {
                run_id: plan.run_id().map(str::to_owned),
                run_success: plan.run_success(),
                attempts,
                entries,
                formatted: plan.format(separator),
            })
        })
    }

    /// Registers a qualified extension after validating its name and
    /// uniqueness.
    ///
    /// # Errors
    /// Invalid/reserved/duplicate names are rejected before registry mutation.
    /// A panic from the supplied name method becomes `INTERNAL_ERROR`.
    pub fn register_extension<E: Extension + 'static>(
        &mut self,
        extension: E,
    ) -> Result<(), SdkError> {
        let name = guarded(|| {
            let name = extension.name();
            let segments = name.split('.').collect::<Vec<_>>();
            if segments.len() < 2
                || segments.iter().any(|segment| !identifier(segment))
            {
                return Err(invalid(
                    "extension name requires namespace.operation identifiers",
                ));
            }
            if segments[0] == "core" {
                return Err(invalid(
                    "the core extension namespace is reserved",
                ));
            }
            if self.extensions.contains_key(name) {
                return Err(invalid("extension name is already registered"));
            }
            Ok(name.to_owned())
        })?;
        self.extensions.insert(name, Box::new(extension));
        Ok(())
    }

    /// Invokes a registered Rust extension without widening core JSON
    /// operations.
    ///
    /// # Errors
    /// Missing names produce `UNKNOWN_OPERATION`; ordinary extension panics are
    /// caught as `INTERNAL_ERROR`. Extension-owned errors pass through
    /// unchanged.
    pub fn execute_extension(
        &self,
        name: &str,
        input: &Value,
    ) -> Result<Value, SdkError> {
        guarded(|| {
            self.extensions
                .get(name)
                .ok_or_else(|| {
                    SdkError::new(
                        ErrorCode::UnknownOperation,
                        "extension is not registered",
                    )
                })?
                .execute(input)
        })
    }

    fn dispatch(&self, text: &str) -> Result<Value, SdkError> {
        let request = json_input::parse(text)
            .map_err(|error| invalid(error.to_string()))?;
        let object = request
            .as_object()
            .ok_or_else(|| invalid("request must be an object"))?;
        let version = object
            .get("version")
            .and_then(Value::as_number)
            .ok_or_else(|| invalid("version must be an integer"))?;
        match version_kind(&version.to_string()) {
            Version::One => {}
            Version::OtherInteger => {
                return Err(SdkError::new(
                    ErrorCode::UnsupportedVersion,
                    "unsupported protocol version",
                ));
            }
            Version::NonInteger => {
                return Err(invalid("version must be an integer"));
            }
        }
        let operation = string(object, "operation")?;
        if !matches!(operation, "capabilities" | "compile" | "rerun") {
            return Err(SdkError::new(
                ErrorCode::UnknownOperation,
                "unknown core operation",
            ));
        }
        exact_fields(object, &["version", "operation", "input"])?;
        let input = object
            .get("input")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("input must be an object"))?;
        match operation {
            "capabilities" => {
                exact_fields(input, &[])?;
                Ok(self.capabilities())
            }
            "compile" => {
                exact_fields(
                    input,
                    &["source", "uri", "format", "defaultLanguage"],
                )?;
                let format = match string(input, "format")? {
                    "gherkin" => SourceFormat::Gherkin,
                    "markdown" => SourceFormat::Markdown,
                    _ => {
                        return Err(invalid(
                            "format must be gherkin or markdown",
                        ));
                    }
                };
                let result = self.compile(CompileInput {
                    source: string(input, "source")?,
                    uri: string(input, "uri")?,
                    format,
                    default_language: string(input, "defaultLanguage")?,
                })?;
                tagged("compiled", &result)
            }
            "rerun" => {
                exact_fields(input, &["ndjson", "separator"])?;
                let result = self.rerun(
                    string(input, "ndjson")?,
                    string(input, "separator")?,
                )?;
                tagged("rerun", &result)
            }
            _ => unreachable!("operation validated above"),
        }
    }
}

fn guarded<T>(
    operation: impl FnOnce() -> Result<T, SdkError>,
) -> Result<T, SdkError> {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or_else(|_| {
        Err(SdkError::new(
            ErrorCode::InternalError,
            "native SDK operation panicked",
        ))
    })
}

fn invalid(message: impl Into<String>) -> SdkError {
    SdkError::new(ErrorCode::InvalidRequest, message)
}

fn message_error(message: impl Into<String>, line: usize) -> SdkError {
    let mut error = SdkError::new(ErrorCode::InvalidMessages, message);
    error.line = Some(line.to_string());
    error
}

fn exact_fields(
    object: &Map<String, Value>,
    names: &[&str],
) -> Result<(), SdkError> {
    if object.len() != names.len()
        || names.iter().any(|name| !object.contains_key(*name))
    {
        return Err(invalid("object contains missing or additional fields"));
    }
    Ok(())
}

fn string<'a>(
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, SdkError> {
    object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{name} must be a string")))
}

fn tagged(
    kind: &str,
    value: &impl serde::Serialize,
) -> Result<Value, SdkError> {
    let mut result = serde_json::to_value(value).map_err(|error| {
        SdkError::new(ErrorCode::InternalError, error.to_string())
    })?;
    result["kind"] = Value::String(kind.to_owned());
    Ok(result)
}

fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

enum Version {
    One,
    OtherInteger,
    NonInteger,
}

// Classify the exact decimal spelling, without a lossy f64 conversion or an
// allocation proportional to an exponent. JSON Schema treats 1.0 and 1e0 as 1.
fn version_kind(number: &str) -> Version {
    let (mantissa, exponent) =
        number.split_once(['e', 'E']).unwrap_or((number, "0"));
    let negative = mantissa.starts_with('-');
    let unsigned = mantissa.trim_start_matches('-');
    let fractional_digits =
        unsigned.split_once('.').map_or(0, |(_, fraction)| fraction.len());
    let digits = unsigned.chars().filter(|c| *c != '.').collect::<String>();
    let significant = digits.trim_start_matches('0');
    if significant.is_empty() {
        return Version::OtherInteger;
    }
    let trailing_zeros =
        significant.len() - significant.trim_end_matches('0').len();
    let exponent = exponent.parse::<i64>().unwrap_or_else(|_| {
        if exponent.starts_with('-') { i64::MIN } else { i64::MAX }
    });
    let scale = exponent
        .saturating_sub(fractional_digits as i64)
        .saturating_add(trailing_zeros as i64);
    if scale < 0 {
        return Version::NonInteger;
    }
    if !negative && scale == 0 && significant.trim_end_matches('0') == "1" {
        Version::One
    } else {
        Version::OtherInteger
    }
}
