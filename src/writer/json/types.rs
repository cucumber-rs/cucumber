// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Basic serializable types for Cucumber JSON format output.

use std::sync::LazyLock;

use base64::Engine as _;
use derive_more::with_trait::Display;
use mime::Mime;
use serde::Serialize;
use serde_with::{DisplayFromStr, serde_as};

/// [`base64`] encoded data.
#[derive(Clone, Debug, Display, Serialize)]
#[serde(transparent)]
pub struct Base64(String);

impl Base64 {
    /// Used [`base64::engine`].
    const ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::STANDARD;

    /// Encodes `bytes` as [`base64`].
    #[must_use]
    pub fn encode(bytes: impl AsRef<[u8]>) -> Self {
        Self(Self::ENGINE.encode(bytes))
    }

    /// Decodes this [`base64`] encoded data.
    #[must_use]
    pub fn decode(&self) -> Vec<u8> {
        Self::ENGINE.decode(&self.0).unwrap_or_else(|_| {
            unreachable!(
                "the only way to construct this type is `Base64::encode`, so \
                 should contain a valid `base64` encoded `String`",
            )
        })
    }
}

/// Data embedded to [Cucumber JSON format][1] output.
///
/// [1]: https://github.com/cucumber/cucumber-json-schema
#[serde_as]
#[derive(Clone, Debug, Serialize)]
pub struct Embedding {
    /// [`base64`] encoded data.
    pub data: Base64,

    /// [`Mime`] of this [`Embedding::data`].
    #[serde_as(as = "DisplayFromStr")]
    pub mime_type: Mime,

    /// Optional name of the [`Embedding`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Embedding {
    /// Creates [`Embedding`] from the provided [`event::Scenario::Log`].
    pub fn from_log(msg: impl AsRef<str>) -> Self {
        /// [`Mime`] of the [`event::Scenario::Log`] [`Embedding`].
        static LOG_MIME: LazyLock<Mime> = LazyLock::new(|| {
            "text/x.cucumber.log+plain"
                .parse()
                .unwrap_or_else(|_| unreachable!("valid MIME"))
        });

        Self {
            data: Base64::encode(msg.as_ref()),
            mime_type: LOG_MIME.clone(),
            name: None,
        }
    }
}

/// [`Serialize`]able tag of a [`gherkin::Feature`] or a [`gherkin::Scenario`].
#[derive(Clone, Debug, Serialize)]
pub struct Tag {
    /// Name of this [`Tag`].
    pub name: String,

    /// Line number of this [`Tag`] in a `.feature` file.
    ///
    /// As [`gherkin`] parser omits this info, line number is taken from
    /// [`gherkin::Feature`] or [`gherkin::Scenario`].
    pub line: usize,
}

/// Possible statuses of running [`gherkin::Step`].
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// [`event::Step::Passed`].
    Passed,

    /// [`event::Step::Failed`] with an [`event::StepError::Panic`].
    Failed,

    /// [`event::Step::Skipped`].
    Skipped,

    /// [`event::Step::Failed`] with an [`event::StepError::AmbiguousMatch`].
    Ambiguous,

    /// [`event::Step::Failed`] with an [`event::StepError::NotFound`].
    Undefined,

    /// Never constructed and is here only to fully describe [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    Pending,
}

/// [`Serialize`]able result of running something.
#[derive(Clone, Debug, Serialize)]
pub struct RunResult {
    /// [`Status`] of this running result.
    pub status: Status,

    /// Execution time.
    ///
    /// While nowhere being documented, [`cucumber-jvm` uses nanoseconds][1].
    ///
    /// [1]: https://tinyurl.com/34wry46u#L325
    pub duration: u128,

    /// Error message of [`Status::Failed`] or [`Status::Ambiguous`] (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// [`Serialize`]able [`gherkin::Step`].
#[derive(Clone, Debug, Serialize)]
pub struct Step {
    /// [`gherkin::Step::keyword`].
    pub keyword: String,

    /// [`gherkin::Step`] line number in a `.feature` file.
    pub line: usize,

    /// [`gherkin::Step::value`].
    pub name: String,

    /// Never [`true`] and is here only to fully describe a [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,

    /// [`RunResult`] of this [`Step`].
    pub result: RunResult,

    /// [`Embedding`]s of this [`Step`].
    ///
    /// Although this field isn't present in the [JSON schema][1], all major
    /// implementations have it (see [Java], [JavaScript], [Ruby]).
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [Java]: https://bit.ly/3J66vxT
    /// [JavaScript]: https://bit.ly/41HSTAf
    /// [Ruby]: https://bit.ly/3kAJRof
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeddings: Vec<Embedding>,
}

/// [`Serialize`]able result of running a [`Before`] or [`After`] hook.
///
/// [`Before`]: crate::event::HookType::Before
/// [`After`]: crate::event::HookType::After
#[derive(Clone, Debug, Serialize)]
pub struct HookResult {
    /// [`RunResult`] of the hook.
    pub result: RunResult,

    /// [`Embedding`]s of this [`Hook`].
    ///
    /// Although this field isn't present in [JSON schema][1], all major
    /// implementations have it (see [Java], [JavaScript], [Ruby]).
    ///
    /// [`Hook`]: crate::event::Hook
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [Java]: https://bit.ly/3J66vxT
    /// [JavaScript]: https://bit.ly/41HSTAf
    /// [Ruby]: https://bit.ly/3kAJRof
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeddings: Vec<Embedding>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_decode_roundtrip() {
        let data = b"Hello, World!";
        let encoded = Base64::encode(data);
        let decoded = encoded.decode();
        assert_eq!(data.as_slice(), decoded.as_slice());
    }

    #[test]
    fn base64_display() {
        let encoded = Base64::encode("test");
        assert_eq!(encoded.to_string(), "dGVzdA==");
    }

    #[test]
    fn embedding_from_log() {
        let msg = "Test log message";
        let embedding = Embedding::from_log(msg);
        
        assert_eq!(embedding.data.decode(), msg.as_bytes());
        assert_eq!(embedding.mime_type.to_string(), "text/x.cucumber.log+plain");
        assert!(embedding.name.is_none());
    }

    #[test]
    fn status_serialization() {
        use serde_json;
        
        assert_eq!(serde_json::to_string(&Status::Passed).unwrap(), "\"passed\"");
        assert_eq!(serde_json::to_string(&Status::Failed).unwrap(), "\"failed\"");
        assert_eq!(serde_json::to_string(&Status::Skipped).unwrap(), "\"skipped\"");
        assert_eq!(serde_json::to_string(&Status::Ambiguous).unwrap(), "\"ambiguous\"");
        assert_eq!(serde_json::to_string(&Status::Undefined).unwrap(), "\"undefined\"");
        assert_eq!(serde_json::to_string(&Status::Pending).unwrap(), "\"pending\"");
    }

    #[test]
    fn run_result_serialization() {
        let result = RunResult {
            status: Status::Passed,
            duration: 12345,
            error_message: None,
        };
        
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["status"], "passed");
        assert_eq!(json["duration"], 12345);
        assert!(!json.as_object().unwrap().contains_key("error_message"));
    }

    #[test]
    fn run_result_with_error_serialization() {
        let result = RunResult {
            status: Status::Failed,
            duration: 54321,
            error_message: Some("Something went wrong".to_string()),
        };
        
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["status"], "failed");
        assert_eq!(json["duration"], 54321);
        assert_eq!(json["error_message"], "Something went wrong");
    }

    #[test]
    fn step_hidden_field_serialization() {
        let step = Step {
            keyword: "Given".to_string(),
            line: 10,
            name: "a test step".to_string(),
            hidden: false,
            result: RunResult {
                status: Status::Passed,
                duration: 1000,
                error_message: None,
            },
            embeddings: vec![],
        };
        
        let json = serde_json::to_value(&step).unwrap();
        // hidden field should be omitted when false
        assert!(!json.as_object().unwrap().contains_key("hidden"));
    }

    #[test]
    fn tag_creation() {
        let tag = Tag {
            name: "@smoke".to_string(),
            line: 5,
        };
        
        assert_eq!(tag.name, "@smoke");
        assert_eq!(tag.line, 5);
    }
}