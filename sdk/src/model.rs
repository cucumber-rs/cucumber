use serde::Serialize;
use serde_json::Value;

/// Explicit parser selection, independent of URI suffix or filesystem state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    Gherkin,
    Markdown,
}

/// Borrowed source compilation request with an explicit default dialect.
#[derive(Clone, Copy, Debug)]
pub struct CompileInput<'a> {
    pub source: &'a str,
    pub uri: &'a str,
    pub format: SourceFormat,
    pub default_language: &'a str,
}

/// Source-faithful graph copied directly from the core compiler.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compiled {
    pub uri: String,
    pub format: SourceFormat,
    pub gherkin_document: Value,
    pub pickles: Vec<Value>,
}

/// Every observed attempt, including passing and subsequently retried attempts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub started_id: String,
    pub test_case_id: String,
    pub pickle_id: String,
    /// Exact decimal representation of the original u64 attempt counter.
    pub attempt: String,
    pub uri: String,
    /// Original source declaration or Examples body-row line, as decimal text.
    pub line: String,
    pub status: String,
    pub completed: bool,
    pub will_be_retried: bool,
}

/// One ordered URI group, retaining duplicate selected lines.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Entry {
    pub uri: String,
    pub lines: Vec<String>,
}

/// A completed run's lossless attempts and derived rerun projection.
///
/// An empty entries vector means no scenarios to rerun, never all scenarios.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RerunPlan {
    pub run_id: Option<String>,
    pub run_success: bool,
    pub attempts: Vec<Attempt>,
    pub entries: Vec<Entry>,
    pub formatted: String,
}
