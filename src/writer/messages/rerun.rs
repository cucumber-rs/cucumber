//! Programmatic rerun plans from actual standard Messages envelopes.
//!
//! Matches the Cucumber.js 13.3 formatter's non-PASSED/not-retried selection.
//! Original attempt history remains separate from the derived source selection.
//! This consumer does not rerun code, rewrite reports, or infer native positions.
use crate::{filter::LineSelectors, parser::SourcePaths};
use serde_json::Value;
use std::{collections::HashMap, fmt, path::PathBuf};

/// Protocol status in the pinned Messages severity order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Status {
    /// No completed aggregate result was observed.
    Unknown,
    /// All relevant completed work passed.
    Passed,
    /// Work was skipped, including dry-run decisions.
    Skipped,
    /// Work was marked pending, independent of strictness.
    Pending,
    /// A step had no matching definition.
    Undefined,
    /// A step matched multiple definitions.
    Ambiguous,
    /// A body or hook failed.
    Failed,
}
impl Status {
    fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "UNKNOWN" => Ok(Self::Unknown),
            "PASSED" => Ok(Self::Passed),
            "SKIPPED" => Ok(Self::Skipped),
            "PENDING" => Ok(Self::Pending),
            "UNDEFINED" => Ok(Self::Undefined),
            "AMBIGUOUS" => Ok(Self::Ambiguous),
            "FAILED" => Ok(Self::Failed),
            _ => Err(Error::InvalidEnvelope("unknown test-step result status")),
        }
    }
}
/// Every observed attempt, including passing and subsequently retried attempts.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Attempt {
    /// Actual TestCaseStarted ID; no replacement identity is generated.
    pub started_id: String,
    /// Actual TestCase ID.
    pub test_case_id: String,
    /// Actual Pickle ID.
    pub pickle_id: String,
    /// Zero-based protocol retry attempt number.
    pub attempt: u64,
    /// Exact source URI from the Pickle.
    pub uri: String,
    /// Original scenario declaration or Examples body-row line.
    pub line: usize,
    /// Worst result computed at TestCaseFinished, or UNKNOWN until then.
    pub status: Status,
    /// Whether TestCaseFinished was actually received.
    pub completed: bool,
    /// Actual retry decision, defaulting to false for interrupted started cases.
    pub will_be_retried: bool,
}
impl Attempt {
    /// Whether this attempt contributes a location to the pinned rerun formatter.
    /// This is independent of run success, pending strictness and dry-run policy.
    #[must_use]
    pub const fn is_selected(&self) -> bool {
        !matches!(self.status, Status::Passed) && !self.will_be_retried
    }
}
/// One URI group in formatter encounter order; duplicate lines are preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Entry {
    /// Exact original URI, not normalized or split on colons.
    pub uri: String,
    /// One original line for each selected attempt, preserving duplicate entries.
    pub lines: Vec<usize>,
}
/// An immutable projection of one explicitly finished protocol run.
#[derive(Clone, Debug)]
pub struct Plan {
    run_id: Option<String>,
    success: bool,
    attempts: Vec<Attempt>,
    entries: Vec<Entry>,
}
impl Plan {
    /// Returns the actual TestRunStarted ID, absent in permitted older streams.
    #[must_use]
    pub fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }
    /// Returns actual run success, even when no scenario location is selectable.
    #[must_use]
    pub const fn run_success(&self) -> bool {
        self.success
    }
    /// Returns every attempt in TestCaseStarted encounter order.
    #[must_use]
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }
    /// Returns the derived URI/line groups without discarding attempt evidence.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// Formats URI:line:line groups with the requested separator and no suffix.
    /// Use "\n" for the pinned formatter's default. Paths are not shell-escaped.
    #[must_use]
    pub fn format(&self, separator: &str) -> String {
        self.entries
            .iter()
            .map(|entry| {
                format!(
                    "{}:{}",
                    entry.uri,
                    entry
                        .lines
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join(":")
                )
            })
            .collect::<Vec<_>>()
            .join(separator)
    }
    /// Resolves a nonempty rerun plan to explicit discovery and source selectors.
    ///
    /// None means nothing to rerun; the resolver is never called in that case.
    /// Apply both returned components to preserve the finite source selection.
    /// URI interpretation belongs to the application; no fallback path is guessed.
    ///
    /// # Errors
    /// Reports URI resolution failures or invalid line-selector configuration.
    pub fn local_selection<F, E>(
        &self,
        mut resolve_uri: F,
    ) -> Result<Option<LocalSelection>, Error>
    where
        F: FnMut(&str) -> Result<PathBuf, E>,
        E: fmt::Display,
    {
        if self.entries.is_empty() {
            return Ok(None);
        }
        let mut paths = SourcePaths::new();
        let mut lines = LineSelectors::new();
        for entry in &self.entries {
            let path =
                resolve_uri(&entry.uri).map_err(|error| Error::Resolve {
                    uri: entry.uri.clone(),
                    message: error.to_string(),
                })?;
            lines = lines
                .lines(&path, entry.lines.iter().copied())
                .map_err(|error| Error::LineSelector(error.to_string()))?;
            paths = paths.path(path);
        }
        Ok(Some(LocalSelection { paths, lines }))
    }
}
/// Nonempty local rerun selection, pairing finite discovery with line filtering.
#[derive(Clone, Debug)]
pub struct LocalSelection {
    paths: SourcePaths,
    lines: LineSelectors,
}
impl LocalSelection {
    /// Takes both components for with_source_paths and with_line_selectors.
    #[must_use]
    pub fn into_parts(self) -> (SourcePaths, LineSelectors) {
        (self.paths, self.lines)
    }
}
/// An incomplete/malformed stream or an application URI-resolution failure.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// No TestRunFinished boundary has been observed.
    IncompleteRun,
    /// Input arrived after the completed run boundary.
    RunAlreadyFinished,
    /// A relevant envelope violates the required field/sequence contract.
    InvalidEnvelope(&'static str),
    /// A referenced identity or source AST node is absent.
    MissingReference {
        /// Referenced object type, such as testCase, pickle, document or astNode.
        kind: &'static str,
        /// Missing original identity.
        id: String,
    },
    /// The application rejected resolution of this original URI.
    Resolve {
        /// URI that could not be resolved.
        uri: String,
        /// Original resolver diagnostic.
        message: String,
    },
    /// A resolved path or source-line selection was invalid.
    LineSelector(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompleteRun => {
                f.write_str("rerun plan requires testRunFinished")
            }
            Self::RunAlreadyFinished => {
                f.write_str("rerun input follows testRunFinished")
            }
            Self::InvalidEnvelope(message) => {
                write!(f, "invalid rerun envelope: {message}")
            }
            Self::MissingReference { kind, id } => {
                write!(f, "missing {kind} reference: {id}")
            }
            Self::Resolve { uri, message } => {
                write!(f, "cannot resolve rerun URI {uri}: {message}")
            }
            Self::LineSelector(message) => {
                write!(f, "invalid rerun line selector: {message}")
            }
        }
    }
}
impl std::error::Error for Error {}
#[derive(Debug)]
struct Pickle {
    uri: String,
    last_ast_id: String,
}
#[derive(Debug)]
struct Case {
    pickle_id: String,
    steps: Vec<String>,
}
#[derive(Debug)]
struct Started {
    id: String,
    case_id: String,
    attempt: u64,
    steps: HashMap<String, Status>,
    worst: Status,
    completed: bool,
    retried: bool,
}
/// Incremental pure-Rust consumer of one Messages run, without filesystem I/O.
///
/// Native source-backed Messages and external standard streams use the same
/// path. Actual parse/global-hook errors remain in their original stream; run
/// success is exposed independently of scenario rerun membership. Unrelated
/// envelopes are ignored. Use a fresh collector for each run.
#[derive(Debug, Default)]
pub struct Collector {
    documents: HashMap<String, HashMap<String, usize>>,
    pickles: HashMap<String, Pickle>,
    cases: HashMap<String, Case>,
    attempts: Vec<Started>,
    indices: HashMap<String, usize>,
    run_started: bool,
    run_id: Option<String>,
    success: Option<bool>,
}
impl Collector {
    /// Creates an empty single-run collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Consumes relevant standard-envelope data without changing the input.
    ///
    /// # Errors
    /// Reports missing references, malformed relevant fields, duplicate attempt
    /// IDs or input after the run boundary. Stop using this collector on error.
    pub fn ingest(&mut self, envelope: &Value) -> Result<(), Error> {
        if self.success.is_some() {
            return Err(Error::RunAlreadyFinished);
        }
        if let Some(document) = envelope.get("gherkinDocument") {
            let uri = text(document, "uri")?.to_owned();
            let mut locations = HashMap::new();
            if let Some(feature) = document.get("feature") {
                collect_children(array(feature, "children")?, &mut locations)?;
            }
            self.documents.insert(uri, locations);
        } else if let Some(pickle) = envelope.get("pickle") {
            let id = text(pickle, "id")?.to_owned();
            let last_ast_id = array(pickle, "astNodeIds")?
                .last()
                .and_then(Value::as_str)
                .ok_or(Error::InvalidEnvelope(
                    "pickle requires a last astNodeId",
                ))?
                .to_owned();
            self.pickles.insert(
                id,
                Pickle { uri: text(pickle, "uri")?.to_owned(), last_ast_id },
            );
        } else if let Some(case) = envelope.get("testCase") {
            let steps = array(case, "testSteps")?
                .iter()
                .map(|step| text(step, "id").map(str::to_owned))
                .collect::<Result<Vec<_>, _>>()?;
            self.cases.insert(
                text(case, "id")?.to_owned(),
                Case { pickle_id: text(case, "pickleId")?.to_owned(), steps },
            );
        } else if let Some(run) = envelope.get("testRunStarted") {
            if self.run_started {
                return Err(Error::InvalidEnvelope(
                    "multiple testRunStarted boundaries",
                ));
            }
            self.run_id = run
                .get("id")
                .map(|id| {
                    id.as_str().map(str::to_owned).ok_or(
                        Error::InvalidEnvelope("invalid testRunStarted id"),
                    )
                })
                .transpose()?;
            self.run_started = true;
        } else if let Some(start) = envelope.get("testCaseStarted") {
            let id = text(start, "id")?.to_owned();
            if self.indices.contains_key(&id) {
                return Err(Error::InvalidEnvelope(
                    "duplicate testCaseStarted identity",
                ));
            }
            let case_id = text(start, "testCaseId")?.to_owned();
            let case = self
                .cases
                .get(&case_id)
                .ok_or_else(|| missing("testCase", &case_id))?;
            let attempt = start.get("attempt").and_then(Value::as_u64).ok_or(
                Error::InvalidEnvelope("testCaseStarted requires attempt"),
            )?;
            self.indices.insert(id.clone(), self.attempts.len());
            self.attempts.push(Started {
                id,
                case_id,
                attempt,
                steps: case
                    .steps
                    .iter()
                    .map(|id| (id.clone(), Status::Unknown))
                    .collect(),
                worst: Status::Unknown,
                completed: false,
                retried: false,
            });
        } else if let Some(step) = envelope.get("testStepFinished") {
            let id = text(step, "testCaseStartedId")?;
            let index = *self
                .indices
                .get(id)
                .ok_or_else(|| missing("testCaseStarted", id))?;
            let attempt = &mut self.attempts[index];
            if attempt.completed {
                return Err(Error::InvalidEnvelope(
                    "step result follows case finish",
                ));
            }
            let step_id = text(step, "testStepId")?;
            let status = attempt
                .steps
                .get_mut(step_id)
                .ok_or_else(|| missing("testStep", step_id))?;
            *status = Status::parse(text(
                step.get("testStepResult")
                    .ok_or(Error::InvalidEnvelope("missing testStepResult"))?,
                "status",
            )?)?;
        } else if let Some(finish) = envelope.get("testCaseFinished") {
            let id = text(finish, "testCaseStartedId")?;
            let index = *self
                .indices
                .get(id)
                .ok_or_else(|| missing("testCaseStarted", id))?;
            let attempt = &mut self.attempts[index];
            if attempt.completed {
                return Err(Error::InvalidEnvelope("case finished twice"));
            }
            attempt.retried = boolean(finish, "willBeRetried")?;
            attempt.worst = attempt
                .steps
                .values()
                .copied()
                .max()
                .unwrap_or(Status::Unknown);
            attempt.completed = true;
        } else if let Some(finish) = envelope.get("testRunFinished") {
            if !self.run_started {
                return Err(Error::InvalidEnvelope(
                    "run finished without testRunStarted",
                ));
            }
            if let Some(id) = finish.get("testRunStartedId") {
                if id.as_str() != self.run_id.as_deref()
                    || id.as_str().is_none()
                {
                    return Err(Error::InvalidEnvelope(
                        "run finish references another run",
                    ));
                }
            }
            self.success = Some(boolean(finish, "success")?);
        }
        Ok(())
    }
    /// Returns an immutable complete-run projection retaining every attempt.
    ///
    /// # Errors
    /// Requires the run boundary and actual document/Pickle/AST references for
    /// every attempt. Expanded native positions are never used as a fallback.
    pub fn plan(&self) -> Result<Plan, Error> {
        let success = self.success.ok_or(Error::IncompleteRun)?;
        let run_id = self.run_id.clone();
        let mut attempts = Vec::new();
        let mut entries: Vec<Entry> = Vec::new();
        for started in &self.attempts {
            let case = self
                .cases
                .get(&started.case_id)
                .ok_or_else(|| missing("testCase", &started.case_id))?;
            let pickle = self
                .pickles
                .get(&case.pickle_id)
                .ok_or_else(|| missing("pickle", &case.pickle_id))?;
            let document = self
                .documents
                .get(&pickle.uri)
                .ok_or_else(|| missing("document", &pickle.uri))?;
            let line = *document
                .get(&pickle.last_ast_id)
                .ok_or_else(|| missing("astNode", &pickle.last_ast_id))?;
            let attempt = Attempt {
                started_id: started.id.clone(),
                test_case_id: started.case_id.clone(),
                pickle_id: case.pickle_id.clone(),
                attempt: started.attempt,
                uri: pickle.uri.clone(),
                line,
                status: started.worst,
                completed: started.completed,
                will_be_retried: started.retried,
            };
            if attempt.is_selected() {
                if let Some(entry) =
                    entries.iter_mut().find(|entry| entry.uri == attempt.uri)
                {
                    entry.lines.push(line);
                } else {
                    entries.push(Entry {
                        uri: attempt.uri.clone(),
                        lines: vec![line],
                    });
                }
            }
            attempts.push(attempt);
        }
        // JavaScript Object.keys orders canonical uint32 index keys before other
        // URI keys. Ordinary path URIs remain in insertion order.
        entries.sort_by_key(|entry| {
            object_index(&entry.uri).map_or((1, 0), |index| (0, index))
        });
        Ok(Plan { run_id, success, attempts, entries })
    }
}
fn object_index(value: &str) -> Option<u32> {
    let index = value.parse::<u32>().ok()?;
    (index != u32::MAX && index.to_string() == value).then_some(index)
}
fn missing(kind: &'static str, id: &str) -> Error {
    Error::MissingReference { kind, id: id.to_owned() }
}
fn text<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, Error> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(Error::InvalidEnvelope(field))
}
fn array<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a Vec<Value>, Error> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or(Error::InvalidEnvelope(field))
}
fn boolean(value: &Value, field: &'static str) -> Result<bool, Error> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or(Error::InvalidEnvelope(field))
}
fn collect_children(
    children: &[Value],
    locations: &mut HashMap<String, usize>,
) -> Result<(), Error> {
    for child in children {
        if let Some(rule) = child.get("rule") {
            collect_children(array(rule, "children")?, locations)?;
        } else if let Some(scenario) = child.get("scenario") {
            location(scenario, locations)?;
            for examples in array(scenario, "examples")? {
                for row in array(examples, "tableBody")? {
                    location(row, locations)?;
                }
            }
        }
    }
    Ok(())
}
fn location(
    value: &Value,
    locations: &mut HashMap<String, usize>,
) -> Result<(), Error> {
    let id = text(value, "id")?.to_owned();
    let line = value
        .get("location")
        .and_then(|location| location.get("line"))
        .and_then(Value::as_u64)
        .and_then(|line| usize::try_from(line).ok())
        .filter(|line| *line > 0)
        .ok_or(Error::InvalidEnvelope(
            "AST location requires a positive line",
        ))?;
    if locations.insert(id, line).is_some() {
        return Err(Error::InvalidEnvelope("duplicate AST location identity"));
    }
    Ok(())
}
