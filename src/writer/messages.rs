//! Standard Cucumber Messages 34.2.1 NDJSON for a deliberately bounded subset.
//!
//! Unlike the legacy JSON writer, this writer emits protocol envelopes, not
//! the legacy
//! Cucumber JSON report. Shared source snapshots enable rich English Gherkin
//! graphs; the original constructors retain their bounded plain-scenario mode.
//! See `docs/compatibility/messages-source.md` for the verified source contract
//! and `docs/compatibility/messages.md` for the original constructor limits.

pub mod rerun;
mod scenario_hooks;
pub mod source;

use std::{
    collections::{HashMap, HashSet},
    fmt, io,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::Engine as _;
use serde_json::{Value, json};

use crate::{
    Event, World, Writer, cli, event, parser,
    writer::{self, Ext as _},
};

/// Explicit standard protocol version for the supported envelope subset.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ProtocolVersion {
    /// Messages 34.2.1, pinned by Cucumber.js 13.3.0 (the default).
    #[default]
    V34,
    /// Messages 35.0.0, used by official Gherkin 43.0.0.
    V35,
}

impl ProtocolVersion {
    /// Returns the exact protocol schema version emitted in metadata.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V34 => "34.2.1",
            Self::V35 => "35.0.0",
        }
    }
}

/// A standard NDJSON writer with source, AST, pickle and execution envelopes.
///
/// Prefer [`crate::Cucumber::with_messages()`] or
/// [`Messages::with_source_store()`]
/// for source-faithful graphs. Unsupported events fail visibly without
/// successful
/// run completion. Raw writers require normalized events. The original
/// [`Messages::new()`] constructor retains restricted source rereading
/// behavior;
/// its files must remain readable and semantically unchanged during the run.
#[derive(Debug)]
pub struct Messages<Out: io::Write> {
    output: Out,
    protocol: ProtocolVersion,
    features: HashMap<String, FeaturePlan>,
    source_store: Option<parser::SourceStore>,
    attachment_store: Option<crate::attachment::AttachmentStore>,
    support_code_store: Option<crate::support_code::Store>,
    support_code: Option<std::sync::Arc<crate::support_code::Snapshot>>,
    scenario_hook_store: Option<crate::runner::ScenarioHookStore>,
    emitted_scenario_hooks: HashSet<u64>,
    emitted_initialization: bool,
    run_hook_store: Option<crate::runner::RunHookStore>,
    run_hook_run: Option<u64>,
    emitted_run_hooks: HashSet<String>,
    emitted_run_definitions: HashSet<u64>,
    emitted_sources: HashSet<String>,
    pending: Vec<Value>,
    read_errors: Vec<String>,
    active: Option<CaseRun>,
    attempts: HashMap<String, Option<event::Retries>>,
    emitted_cases: HashMap<String, Vec<Value>>,
    emitted_hooks: [bool; 2],
    started: bool,
    finished: bool,
    success: bool,
    poisoned: bool,
}

#[derive(Debug)]
struct FeaturePlan {
    scenarios: Vec<CasePlan>,
}

#[derive(Clone, Debug)]
struct CasePlan {
    id: String,
    pickle_id: String,
    steps: Vec<gherkin::Step>,
    test_steps: Vec<Value>,
    scenario: gherkin::Scenario,
    rule: Option<gherkin::Rule>,
}

#[derive(Debug)]
struct CaseRun {
    uri: String,
    plan: CasePlan,
    started_id: String,
    next_step: usize,
    step_started: Option<SystemTime>,
    aborted: bool,
    retries: Option<event::Retries>,
    retryable_failure: bool,
    controlled: bool,
    events: Vec<Value>,
    hooks: [bool; 2],
    hook_started: Option<(usize, SystemTime)>,
}

/// Failure to represent an event faithfully or write its output.
#[derive(Debug)]
pub enum Error {
    /// This bounded writer cannot represent the supplied information.
    Unsupported(&'static str),
    /// The exact source is unavailable or differs from the executed AST.
    Source(String),
    /// Explicit expression metadata disagrees with actual registered matching.
    Metadata(crate::support_code::MetadataError),
    /// Events violate the writer's ordering or identity contract.
    InvalidEvent(&'static str),
    /// Writing the output failed.
    Io(io::Error),
    /// Encoding an envelope failed.
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(s) => {
                write!(f, "unsupported Messages input: {s}")
            }
            Self::Source(s) => {
                write!(f, "Messages source unavailable or changed: {s}")
            }
            Self::Metadata(error) => {
                write!(f, "invalid Messages expression metadata: {error}")
            }
            Self::InvalidEvent(s) => {
                write!(f, "invalid Messages event sequence: {s}")
            }
            Self::Io(e) => write!(f, "failed to write Messages: {e}"),
            Self::Json(e) => write!(f, "failed to encode Messages: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl<Out: io::Write> Messages<Out> {
    /// Creates a writer that normalizes concurrent runner events before output.
    #[must_use]
    pub fn new<W: World>(output: Out) -> writer::Normalize<W, Self> {
        Self::raw(output).normalized()
    }

    /// Creates a normalized reporter with an explicit standard protocol
    /// version.
    #[must_use]
    pub fn with_protocol<W: World>(
        output: Out,
        protocol: ProtocolVersion,
    ) -> writer::Normalize<W, Self> {
        Self::raw_with_protocol(output, protocol).normalized()
    }

    /// Creates an already-normalized-stream reporter with an explicit protocol.
    #[must_use]
    pub fn raw_with_protocol(output: Out, protocol: ProtocolVersion) -> Self {
        Self { protocol, ..Self::raw(output) }
    }

    /// Shares source snapshots and selects an explicit standard protocol
    /// version.
    #[must_use]
    pub fn with_source_store_and_protocol<W: World>(
        output: Out,
        store: parser::SourceStore,
        protocol: ProtocolVersion,
    ) -> writer::Normalize<W, Self> {
        Self::raw_with_source_store_and_protocol(output, store, protocol)
            .normalized()
    }

    /// Source-backed reporter for an already-normalized stream in the chosen
    /// protocol.
    #[must_use]
    pub fn raw_with_source_store_and_protocol(
        output: Out,
        store: parser::SourceStore,
        protocol: ProtocolVersion,
    ) -> Self {
        Self { protocol, ..Self::raw_with_source_store(output, store) }
    }

    /// Creates a source-faithful writer sharing immutable snapshots with the
    /// parser.
    ///
    /// Captured source enables tags, rules, descriptions, outlines and rich
    /// arguments without rereading files or changing the native execution AST.
    #[must_use]
    pub fn with_source_store<W: World>(
        output: Out,
        store: parser::SourceStore,
    ) -> writer::Normalize<W, Self> {
        Self::raw_with_source_store(output, store).normalized()
    }

    /// Creates a source-backed writer for already-normalized event streams.
    #[must_use]
    pub fn raw_with_source_store(
        output: Out,
        store: parser::SourceStore,
    ) -> Self {
        Self { source_store: Some(store), ..Self::raw(output) }
    }

    /// Creates a raw writer for already-normalized events or writer
    /// composition.
    #[must_use]
    pub fn raw(output: Out) -> Self {
        Self {
            output,
            protocol: ProtocolVersion::V34,
            features: HashMap::new(),
            source_store: None,
            attachment_store: None,
            support_code_store: None,
            support_code: None,
            scenario_hook_store: None,
            emitted_scenario_hooks: HashSet::new(),
            emitted_initialization: false,
            run_hook_store: None,
            run_hook_run: None,
            emitted_run_hooks: HashSet::new(),
            emitted_run_definitions: HashSet::new(),
            emitted_sources: HashSet::new(),
            pending: Vec::new(),
            read_errors: Vec::new(),
            active: None,
            attempts: HashMap::new(),
            emitted_cases: HashMap::new(),
            emitted_hooks: [false; 2],
            started: false,
            finished: false,
            success: true,
            poisoned: false,
        }
    }

    /// Consumes typed evidence from the same store configured on the Basic
    /// runner.
    ///
    /// Configure this raw writer before normalizing it. Native tracing logs
    /// remain independent and keep only their observed scenario-attempt
    /// identity.
    #[must_use]
    pub fn with_attachment_store(
        mut self,
        store: crate::attachment::AttachmentStore,
    ) -> Self {
        self.attachment_store = Some(store);
        self
    }

    /// Consumes the actual final registry published by a supporting runner.
    ///
    /// Configure this raw writer before normalizing it. A runner without this
    /// capability leaves the store empty, so definition metadata stays omitted.
    #[must_use]
    pub fn with_support_code_store(
        mut self,
        store: crate::support_code::Store,
    ) -> Self {
        self.support_code_store = Some(store);
        self
    }

    /// Reports actual selected scenario callbacks from the same observed
    /// registry.
    ///
    /// Without this explicit handoff, native stages retain composite reporting.
    /// Use a fresh store for every independent run, shared with
    /// `Hooks::with_store`.
    #[must_use]
    pub fn with_scenario_hook_store(
        mut self,
        store: crate::runner::ScenarioHookStore,
    ) -> Self {
        self.scenario_hook_store = Some(store);
        self
    }

    /// Reports actual global callback invocations from the same native runner.
    ///
    /// Use a fresh store for concurrent independent runs. Sequential store
    /// reuse
    /// is supported; the reporter captures the current run at `Started`.
    #[must_use]
    pub fn with_run_hook_store(
        mut self,
        store: crate::runner::RunHookStore,
    ) -> Self {
        self.run_hook_store = Some(store);
        self
    }

    /// Processes one already-normalized event without panicking on errors.
    ///
    /// After any error, the writer is poisoned and refuses further events.
    /// An error during an unfinished run prevents successful completion. An
    /// invalid extra event after a completed run cannot retract emitted bytes.
    ///
    /// # Errors
    /// Returns an error for unsupported inputs, invalid ordering, or output
    /// I/O.
    pub fn try_handle_event<W>(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
    ) -> Result<(), Error> {
        if self.poisoned {
            return Err(Error::InvalidEvent(
                "writer is poisoned by an earlier error",
            ));
        }
        let result = self.handle(event);
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    fn emit(&mut self, envelope: Value) -> Result<(), Error> {
        serde_json::to_writer(&mut self.output, &envelope)
            .map_err(Error::Json)?;
        self.output.write_all(b"\n").map_err(Error::Io)
    }

    fn handle<W>(
        &mut self,
        input: parser::Result<Event<event::Cucumber<W>>>,
    ) -> Result<(), Error> {
        if self.finished {
            return Err(Error::InvalidEvent("event after completed run"));
        }
        let input = match input {
            Ok(input) => input,
            Err(error) => return self.input_error(error),
        };
        let (ev, meta) = input.split();
        // Parsing and execution are joined concurrently by the native runner;
        // its parsing summary may legitimately precede Started.
        if !self.started
            && !matches!(
                ev,
                event::Cucumber::Started
                    | event::Cucumber::ParsingFinished { .. }
            )
        {
            return Err(Error::InvalidEvent("run has not started"));
        }
        match ev {
            event::Cucumber::Started => {
                if self.started {
                    return Err(Error::InvalidEvent("duplicate run start"));
                }
                self.started = true;
                self.run_hook_run = self
                    .run_hook_store
                    .as_ref()
                    .and_then(crate::runner::RunHookStore::current_run);
                self.emit(json!({"meta": {
                    "protocolVersion": self.protocol.as_str(),
                    "implementation": {
                        "name": "cucumber-rs",
                         "version": env!("CARGO_PKG_VERSION")}
                        ,

                    "runtime": {"name": "Rust"},
                    "os": {"name": std::env::consts::OS},
                    "cpu": {"name": std::env::consts::ARCH}
                }}))?;
                self.support_code = self
                    .support_code_store
                    .as_ref()
                    .and_then(crate::support_code::Store::get);
                if let Some(snapshot) = self.support_code.clone() {
                    let mut parameters = std::collections::BTreeMap::<
                        String,
                        crate::support_code::ParameterDescriptor,
                    >::new();
                    for definition in snapshot.definitions() {
                        let source = definition.location().map_or_else(
                            || json!({}),
                            |location| {
                                json!({"uri": location.path,
                                "location": {
                                    "line": location.line,
                                     "column": location.column}
                                    }
                                    )
                            },
                        );
                        let pattern = if let Some(metadata) =
                            definition.expression_metadata()
                        {
                            for descriptor in metadata.parameters() {
                                if let Some(previous) = parameters.insert(
                                    descriptor.name.clone(),
                                    descriptor.clone(),
                                ) {
                                    if previous != *descriptor {
                                        return Err(Error::Metadata(
                                            crate::support_code::MetadataError(
                                                "expression metadata has \
                                                conflicting parameter \
                                                descriptors"
                                                    .to_owned(),
                                            ),
                                        ));
                                    }
                                }
                            }
                            json!({
                            "source": metadata.source(),
                             "type": "CUCUMBER_EXPRESSION"}
                            )
                        } else {
                            json!({
                            "source": definition.regex().as_str(),
                             "type": "REGULAR_EXPRESSION"}
                            )
                        };
                        self.emit(
                            json!({"stepDefinition": {"id": definition.id(),
                            "pattern": pattern, "sourceReference": source}}),
                        )?;
                    }
                    for (index, descriptor) in parameters.values().enumerate() {
                        self.emit(json!({
                            "parameterType": {
                            "id": format!("parameter-type-{index}"),

                            "name": descriptor.name,
                                 "regularExpressions": descriptor
                                     .regular_expressions,

                            "preferForRegularExpressionMatch": descriptor
                                .prefer_for_regular_expression_match,

                            "useForSnippets": descriptor.use_for_snippets}}))?;
                    }
                }
                self.emit(json!({
                "testRunStarted": {
                "id": "run-0",
                 "timestamp": timestamp(meta.at)?}
                }
                ))?;
                for envelope in std::mem::take(&mut self.pending) {
                    self.emit(envelope)?;
                }
                Ok(())
            }
            event::Cucumber::Feature(feature, event::Feature::Started) => {
                self.run_hooks(crate::runner::RunHookKind::BeforeAll)?;
                self.feature(&feature)
            }
            event::Cucumber::Feature(
                feature,
                event::Feature::Scenario(scenario, ev),
            ) => {
                let uri = feature_uri(&feature)?;
                self.scenario(
                    &uri,
                    None,
                    &scenario,
                    ev.event,
                    ev.retries,
                    meta.at,
                    meta.outcome,
                )
            }
            event::Cucumber::Feature(
                feature,
                event::Feature::Rule(rule, ev),
            ) => {
                if self.source_store.is_none() {
                    return Err(Error::Unsupported("rules"));
                }
                match ev {
                    event::Rule::Scenario(scenario, ev) => self.scenario(
                        &feature_uri(&feature)?,
                        Some(&rule),
                        &scenario,
                        ev.event,
                        ev.retries,
                        meta.at,
                        meta.outcome,
                    ),
                    event::Rule::Started | event::Rule::Finished => Ok(()),
                }
            }
            event::Cucumber::Feature(_, event::Feature::Finished) => {
                if self.active.is_some() {
                    return Err(Error::InvalidEvent(
                        "feature finished with an active scenario",
                    ));
                }
                Ok(())
            }
            event::Cucumber::ParsingFinished { parser_errors, .. } => {
                if parser_errors > 0 {
                    self.success = false;
                }
                Ok(())
            }
            event::Cucumber::Finished => {
                if self.active.is_some() {
                    return Err(Error::InvalidEvent(
                        "run finished with an active scenario",
                    ));
                }
                if self.attempts.values().any(Option::is_some) {
                    return Err(Error::InvalidEvent(
                        "run finished with a pending retry",
                    ));
                }
                if self
                    .attachment_store
                    .as_ref()
                    .is_some_and(|store| !store.is_empty())
                {
                    return Err(Error::InvalidEvent(
                        "attachment store contains unmatched execution \
                            records",
                    ));
                }
                self.run_hooks(crate::runner::RunHookKind::BeforeAll)?;
                self.empty_documents()?;
                self.run_hooks(crate::runner::RunHookKind::AfterAll)?;
                let mut finished = json!({"testRunStartedId": "run-0",
                    "success": self.success, "timestamp": timestamp(meta.at)?});
                if !self.read_errors.is_empty() {
                    finished["exception"] = json!({
                    "type": "InputReadError",
                     "message": self.read_errors.join("\n")}
                    );
                }
                self.emit(json!({"testRunFinished": finished}))?;
                self.output.flush().map_err(Error::Io)?;
                self.finished = true;
                Ok(())
            }
        }
    }

    fn run_hooks(
        &mut self,
        kind: crate::runner::RunHookKind,
    ) -> Result<(), Error> {
        let (records, skipped) = match (&self.run_hook_store, self.run_hook_run)
        {
            (Some(store), Some(run)) => {
                (store.records_for(run), store.skipped_for(run))
            }
            _ => return Ok(()),
        };
        // A run either invokes global callbacks or records dry-run decisions.
        // Preserve the store's setup/reverse-cleanup order, including equal
        // timestamps.
        let observations = records
            .into_iter()
            .map(|record| {
                let status =
                    if record.result.is_ok() { "PASSED" } else { "FAILED" };
                (
                    record.definition,
                    format!(
                        "native-run-hook-invocation-{}",
                        record.invocation_id
                    ),
                    record.started_at,
                    record.finished_at,
                    status,
                    record.result.err(),
                )
            })
            .chain(skipped.into_iter().map(|decision| {
                (
                    decision.definition,
                    format!("native-run-hook-skip-{}", decision.decision_id),
                    decision.at,
                    decision.at,
                    "SKIPPED",
                    None,
                )
            }));
        for (
            definition,
            started_id,
            started_at,
            finished_at,
            status,
            message,
        ) in observations
        {
            if definition.kind != kind
                || !self.emitted_run_hooks.insert(started_id.clone())
            {
                continue;
            }
            let hook_id = format!("native-run-hook-{}", definition.id);
            if self.emitted_run_definitions.insert(definition.id) {
                self.emit(json!({
                    "hook": {
                    "id": hook_id,
                     "name": definition.name,

                    "type": match kind {
                         crate::runner::RunHookKind::BeforeAll => "BEFORE_TES\
                             T_RUN",

                        crate::runner::RunHookKind::AfterAll => "AFTER_TEST_R\
                            UN" }
                            ,

                    "sourceReference": {}}}))?;
            }
            self.emit(json!({"testRunHookStarted": {"id": started_id,
                "testRunStartedId": "run-0", "hookId": hook_id,
                "timestamp": timestamp(started_at)?}}))?;
            let elapsed =
                finished_at.duration_since(started_at).map_err(|_| {
                    Error::InvalidEvent(
                        "global hook finished before it started",
                    )
                })?;
            let mut result =
                json!({"status": status, "duration": duration(elapsed)});
            if let Some(message) = message {
                self.success = false;
                result["message"] = json!(message);
            }
            self.emit(json!({
                "testRunHookFinished": {
                "testRunHookStartedId": started_id,

                "result": result, "timestamp": timestamp(finished_at)?}}))?;
        }
        Ok(())
    }

    fn empty_documents(&mut self) -> Result<(), Error> {
        let documents = self
            .source_store
            .as_ref()
            .map(parser::SourceStore::empty_documents)
            .unwrap_or_default();
        for document in documents {
            let uri = document.path().to_str().ok_or_else(|| {
                Error::Source("a UTF-8 source path is required".into())
            })?;
            if self.features.contains_key(uri) {
                return Err(Error::Source(
                    "featureless source conflicts with \
                    an already reported Feature generation"
                        .into(),
                ));
            }
            let graph = source::SourceGraph::from_empty_document(&document)?;
            self.source(uri, document.source())?;
            self.emit(json!({"gherkinDocument": graph.gherkin_document()}))?;
        }
        Ok(())
    }

    fn emit_or_queue(&mut self, envelope: Value) -> Result<(), Error> {
        if self.started {
            self.emit(envelope)
        } else {
            self.pending.push(envelope);
            Ok(())
        }
    }

    fn source(&mut self, uri: &str, data: &str) -> Result<(), Error> {
        if self.emitted_sources.insert(uri.to_owned()) {
            let format = self
                .source_store
                .as_ref()
                .and_then(|store| store.input_format(std::path::Path::new(uri)))
                .unwrap_or(parser::SourceFormat::Plain);
            let media_type = format.media_type().ok_or_else(|| {
                Error::Source(
                    "source format was not resolved before capture".into(),
                )
            })?;
            self.emit_or_queue(json!({"source": {"uri": uri, "data": data,
                "mediaType": media_type}}))?;
        }
        Ok(())
    }

    fn input_error(&mut self, error: parser::Error) -> Result<(), Error> {
        self.success = false;
        let (path, position, message) = match &error {
            parser::Error::Parsing(error) => match error.as_ref() {
                gherkin::ParseFileError::Reading { source, .. } => {
                    self.read_errors.push(format!("{error}: {source}"));
                    return Ok(());
                }
                gherkin::ParseFileError::Parsing {
                    path,
                    source,
                    error: detail,
                } => {
                    let mut message = format!("{error}: {source}");
                    if let Some(detail) = detail {
                        message.push_str(&format!("; {detail:?}"));
                    }
                    (Some(path), source.position(), message)
                }
            },
            parser::Error::ExampleExpansion(error) => {
                (error.path.as_ref(), error.pos, error.to_string())
            }
        };
        let mut reference = json!({"location": location(position)});
        if let Some(path) = path {
            let uri = path.to_str().ok_or_else(|| {
                Error::Source("a UTF-8 diagnostic path is required".into())
            })?;
            reference["uri"] = json!(uri);
            if let Some(input) =
                self.source_store.as_ref().and_then(|store| store.input(path))
            {
                self.source(uri, &input)?;
            }
        }
        self.emit_or_queue(
            json!({"parseError": {"source": reference, "message": message}}),
        )
    }

    fn feature(&mut self, feature: &gherkin::Feature) -> Result<(), Error> {
        let uri = feature_uri(feature)?;
        if self.active.is_some() || self.features.contains_key(&uri) {
            return Err(Error::InvalidEvent(
                "feature start is repeated or interleaved",
            ));
        }
        if let Some(store) = &self.source_store {
            let snapshot =
                store.get(std::path::Path::new(&uri)).ok_or_else(|| {
                    Error::Source(format!(
                        "missing captured snapshot for {uri}"
                    ))
                })?;
            let original = snapshot.feature();
            if original.keyword != feature.keyword
                || original.name != feature.name
                || original.tags != feature.tags
                || original.description != feature.description
                || original.background != feature.background
            {
                return Err(Error::Source(
                    "runtime feature header/background \
                    differs from captured source"
                        .into(),
                ));
            }
            let graph = source::SourceGraph::from_snapshot(&uri, &snapshot)?;
            let plans = graph
                .cases
                .iter()
                .map(|case| {
                    let pickle_id =
                        case.pickle["id"].as_str().unwrap().to_owned();
                    let id = format!("{pickle_id}-case");
                    let test_steps = case.pickle["steps"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .enumerate()
                        .map(|(i, step)| {
                            json!({
                            "id": format!("{id}-step-{i}"),
                             "pickleStepId": step["id"]}
                            )
                        })
                        .collect();
                    CasePlan {
                        id,
                        pickle_id,
                        test_steps,
                        steps: case.steps.clone(),
                        scenario: case.scenario.clone(),
                        rule: case.rule.clone(),
                    }
                })
                .collect();
            self.source(&uri, snapshot.source())?;
            self.emit(json!({"gherkinDocument": graph.gherkin_document()}))?;
            for pickle in graph.pickles() {
                self.emit(json!({"pickle": pickle}))?;
            }
            self.features.insert(uri, FeaturePlan { scenarios: plans });
            return Ok(());
        }
        let source = std::fs::read_to_string(&uri)
            .map_err(|e| Error::Source(format!("{uri}: {e}")))?;
        let mut original = gherkin::Feature::parse(&source, Default::default())
            .map_err(|e| Error::Source(format!("{uri}: {e}")))?;
        original.path = feature.path.clone();
        validate_source(&original, &source)?;
        if &original != feature {
            return Err(Error::Source(format!(
                "{uri}: parsed source differs from runner AST"
            )));
        }

        let mut children = Vec::new();
        if let Some(background) = &feature.background {
            children.push(json!({"background": {
                "id": ast_id(&uri, "background", background.position),
                "location": location(background.position),
                     "keyword": background.keyword,

                "name": background.name, "description": "",
                "steps": background.steps.iter().map(|s| ast_step(&uri,
                     s)).collect::<Vec<_>>()
            }}));
        }
        let mut plans = Vec::new();
        let mut pickles = Vec::new();
        for scenario in &feature.scenarios {
            let ast_scenario_id = ast_id(&uri, "scenario", scenario.position);
            children.push(json!({"scenario": {
                "id": ast_scenario_id, "location": location(scenario.position),
                "keyword": scenario.keyword,
                     "name": scenario.name,
                     "description": "",

                "tags": [], "examples": [],
                "steps": scenario.steps.iter().map(|s| ast_step(&uri,
                     s)).collect::<Vec<_>>()
            }}));
            let pickle_id = format!("{uri}#pickle-{}", scenario.position.line);
            let case_id = format!("{uri}#case-{}", scenario.position.line);
            let steps = feature
                .background
                .iter()
                .flat_map(|b| b.steps.iter())
                .chain(scenario.steps.iter())
                .cloned()
                .collect::<Vec<_>>();
            let mut pickle_steps = Vec::new();
            let mut test_steps = Vec::new();
            // Match the official Gherkin compiler: conjunction inherits across
            // the background/scenario boundary; wildcard resets to Unknown.
            let mut last_keyword_type = "Unknown";
            for (index, step) in steps.iter().enumerate() {
                let keyword_type = step_keyword_type(step);
                if keyword_type != "Conjunction" {
                    last_keyword_type = keyword_type;
                }
                let pickle_step_id = format!("{pickle_id}-step-{index}");
                pickle_steps.push(json!({
                    "id": pickle_step_id,
                         "astNodeIds": [ast_id(&uri,
                         "step",
                         step.position)],

                    "text": step.value, "type": last_keyword_type
                }));
                // Registry definitions and captures are absent from runner
                // events.
                // Omit their optional protocol fields instead of inventing
                // them.
                test_steps.push(json!({
                "id": format!("{case_id}-step-{index}"),
                 "pickleStepId": pickle_step_id}
                ));
            }
            pickles.push(json!({"pickle": {
                "id": pickle_id,
                     "uri": uri,
                     "name": scenario.name,
                     "language": "en",

                "location": location(scenario.position), "steps": pickle_steps,
                "tags": [], "astNodeIds": [ast_scenario_id]
            }}));
            plans.push(CasePlan {
                id: case_id,
                pickle_id,
                steps,
                test_steps,
                scenario: scenario.clone(),
                rule: None,
            });
        }
        let comments = source
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let trimmed = line.trim_start();
                (trimmed.starts_with('#')
                    && !trimmed.starts_with("# language:"))
                .then(|| json!({"location": {"line": index + 1}, "text": line}))
            })
            .collect::<Vec<_>>();
        self.source(&uri, &source)?;
        self.emit(json!({"gherkinDocument": {
            "uri": uri, "comments": comments,
            "feature": {
                "location": location(feature.position),
                 "tags": [],
                 "language": "en",

                "keyword": feature.keyword,
                     "name": feature.name,
                     "description": "",
                     "children": children}

        }}))?;
        for pickle in pickles {
            self.emit(pickle)?;
        }
        self.features.insert(uri, FeaturePlan { scenarios: plans });
        Ok(())
    }

    fn scenario<W>(
        &mut self,
        uri: &str,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        event: event::Scenario<W>,
        retries: Option<event::Retries>,
        at: SystemTime,
        outcome: Option<crate::execution::Outcome>,
    ) -> Result<(), Error> {
        match event {
            event::Scenario::Started => {
                if self.active.is_some() {
                    return Err(Error::InvalidEvent(
                        "scenario starts while another is active; normalize \
                            events",
                    ));
                }
                let plans = &mut self
                    .features
                    .get_mut(uri)
                    .ok_or(Error::InvalidEvent(
                        "scenario feature has no source plan",
                    ))?
                    .scenarios;
                let matches = plans
                    .iter()
                    .enumerate()
                    .filter(|(_, plan)| {
                        &plan.scenario == scenario
                            && same_rule(plan.rule.as_ref(), rule)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if matches.len() != 1 {
                    return Err(Error::Source(
                        "runtime scenario has no unique \
                        captured example-row identity"
                            .into(),
                    ));
                }
                let plan = plans[matches[0]].clone();
                let attempt = retries.map_or(0, |r| r.current);
                if let Some(expected) = self.attempts.get(&plan.id) {
                    if expected.is_none() || *expected != retries {
                        return Err(Error::InvalidEvent(
                            "unexpected or duplicate retry attempt",
                        ));
                    }
                } else {
                    if attempt != 0 {
                        return Err(Error::InvalidEvent(
                            "first attempt must be zero",
                        ));
                    }
                }
                self.attempts.insert(plan.id.clone(), None);
                let started_id = format!("{}-attempt-{attempt}", plan.id);
                let events = vec![json!({
                        "testCaseStarted": {
                        "id": started_id,
                         "testCaseId": plan.id,

                    "attempt": attempt, "timestamp": timestamp(at)?}})];
                self.active = Some(CaseRun {
                    uri: uri.to_owned(),
                    plan,
                    started_id,
                    next_step: 0,
                    step_started: None,
                    aborted: false,
                    retries,
                    retryable_failure: false,
                    controlled: false,
                    events,
                    hooks: [false; 2],
                    hook_started: None,
                });
                Ok(())
            }
            event::Scenario::Hook(kind, ev) => {
                let mut case = self
                    .active
                    .take()
                    .ok_or(Error::InvalidEvent("hook outside scenario"))?;
                check_case(&case, uri, rule, scenario)?;
                case.controlled |= outcome.is_some();
                if case.retries != retries {
                    return Err(Error::InvalidEvent(
                        "retry metadata changed during attempt",
                    ));
                }
                let index = match kind {
                    event::HookType::Before => 0,
                    event::HookType::After => 1,
                };
                match ev {
                    event::Hook::Started => {
                        if case.hooks[index]
                            || case.hook_started.is_some()
                            || case.step_started.is_some()
                            || (index == 0
                                && (case.next_step != 0 || case.hooks[1]))
                        {
                            return Err(Error::InvalidEvent(
                                "hook stage started out of order",
                            ));
                        }
                        if index == 1 {
                            if (case.controlled || !case.aborted)
                                && case.next_step != case.plan.steps.len()
                            {
                                return Err(Error::InvalidEvent(
                                    "after hook started before body finished",
                                ));
                            }
                            self.skip_remaining(&mut case, at)?;
                        }
                        case.hooks[index] = true;
                        case.hook_started = Some((index, at));
                        case.events.push(json!({
                        "testStepStarted": {
                        "testCaseStartedId": case.started_id,

                        "testStepId": hook_step_id(&case,
                             index),
                             "timestamp": timestamp(at)?}
                            }
                            ));
                    }
                    event::Hook::Passed | event::Hook::Failed(..) => {
                        let (started_index, started) = case
                            .hook_started
                            .take()
                            .ok_or(Error::InvalidEvent(
                                "hook finishes without starting",
                            ))?;
                        if started_index != index {
                            return Err(Error::InvalidEvent(
                                "different hook stage finished",
                            ));
                        }
                        let elapsed =
                            at.duration_since(started).map_err(|_| {
                                Error::InvalidEvent(
                                    "hook timestamp moved backwards",
                                )
                            })?;
                        let mut result = json!({
                        "status": outcome.map_or("PASSED",
                         |outcome| outcome_status(outcome.status)),
                         "duration": duration(elapsed)}
                        );
                        if let event::Hook::Failed(_, info) = ev {
                            if outcome.is_none() {
                                result["status"] = json!("FAILED");
                            }
                            if outcome.is_none_or(|value| {
                                value.status
                                    == crate::execution::OutcomeStatus::Failed
                            }) {
                                result["message"] =
                                    json!(writer::basic::coerce_error(&info));
                            }
                            case.aborted = true;
                            case.retryable_failure =
                                outcome.is_none_or(|value| value.retryable);
                        }
                        case.events.push(json!({
                            "testStepFinished": {
                            "testCaseStartedId": case.started_id,

                            "testStepId": hook_step_id(&case,
                                 index),
                                 "timestamp": timestamp(at)?,

                            "testStepResult": result}}));
                    }
                }
                self.active = Some(case);
                Ok(())
            }
            event::Scenario::Log(body) => {
                let case = self
                    .active
                    .as_mut()
                    .ok_or(Error::InvalidEvent("log outside scenario"))?;
                check_case(case, uri, rule, scenario)?;
                if case.retries != retries {
                    return Err(Error::InvalidEvent(
                        "retry metadata changed during attempt",
                    ));
                }
                // Native logs carry scenario/attempt identity, but no step or
                // hook identity. After-hook logs can precede deferred failure
                // and hook lifecycle events; inferring an active step would
                // misattribute them. Preserve only the observed case boundary.
                case.events.push(json!({"attachment": {
                    "body": body, "contentEncoding": "IDENTITY",
                    "mediaType": "text/x.cucumber.log+plain",
                    "testCaseStartedId": case.started_id,
                    "timestamp": timestamp(at)?
                }}));
                Ok(())
            }
            event::Scenario::Step(step, ev)
            | event::Scenario::Background(step, ev) => {
                let mut case = self
                    .active
                    .take()
                    .ok_or(Error::InvalidEvent("step outside scenario"))?;
                check_case(&case, uri, rule, scenario)?;
                case.controlled |= outcome.is_some();
                if case.retries != retries {
                    return Err(Error::InvalidEvent(
                        "retry metadata changed during attempt",
                    ));
                }
                if (case.aborted && !case.controlled)
                    || case.hook_started.is_some()
                    || case.hooks[1]
                    || case.plan.steps.get(case.next_step)
                        != Some(step.as_ref())
                {
                    return Err(Error::InvalidEvent(
                        "step does not match the next planned pickle step",
                    ));
                }
                match ev {
                    event::Step::Started => {
                        if case.step_started.replace(at).is_some() {
                            return Err(Error::InvalidEvent(
                                "step started twice",
                            ));
                        }
                        self.step_started(&mut case, at)?;
                    }
                    event::Step::Passed(..) => self.step_finished(
                        &mut case,
                        at,
                        outcome.map_or("PASSED", |value| {
                            outcome_status(value.status)
                        }),
                        None,
                    )?,
                    event::Step::Skipped => {
                        // Native Skipped means no matching definition; later
                        // steps
                        // have no events and are mapped to SKIPPED at case
                        // finish.
                        self.step_finished(
                            &mut case,
                            at,
                            outcome.map_or("UNDEFINED", |value| {
                                outcome_status(value.status)
                            }),
                            None,
                        )?;
                        case.aborted = true;
                    }
                    event::Step::Failed(_, _, _, error) => {
                        let status = match &error {
                            event::StepError::NotFound => "UNDEFINED",
                            event::StepError::AmbiguousMatch(_) => "AMBIGUOUS",
                            event::StepError::Panic(_) => "FAILED",
                        };
                        case.retryable_failure = outcome.map_or(
                            !matches!(error, event::StepError::NotFound),
                            |value| value.retryable,
                        );
                        let status = outcome.map_or(status, |value| {
                            outcome_status(value.status)
                        });
                        let message = matches!(
                            status,
                            "FAILED" | "AMBIGUOUS" | "UNDEFINED"
                        )
                        .then(|| error.to_string());
                        self.step_finished(&mut case, at, status, message)?;
                        case.aborted = true;
                    }
                }
                self.active = Some(case);
                Ok(())
            }
            event::Scenario::Finished => {
                let mut case = self.active.take().ok_or(
                    Error::InvalidEvent("scenario finishes without starting"),
                )?;
                check_case(&case, uri, rule, scenario)?;
                case.controlled |= outcome.is_some();
                if case.retries != retries {
                    return Err(Error::InvalidEvent(
                        "retry metadata changed during attempt",
                    ));
                }
                if case.step_started.is_some()
                    || case.hook_started.is_some()
                    || ((case.controlled || !case.aborted)
                        && case.next_step != case.plan.steps.len())
                {
                    return Err(Error::InvalidEvent(
                        "scenario finishes with missing step outcomes",
                    ));
                }
                self.skip_remaining(&mut case, at)?;
                // The native runner retries panics/ambiguity, but not undefined
                // steps. Preserve that observed policy rather than guessing
                // from
                // a generic non-passing status.
                let next = case
                    .retries
                    .filter(|_| {
                        outcome.map_or(case.retryable_failure, |value| {
                            value.retryable
                        })
                    })
                    .and_then(event::Retries::next_try);
                self.attempts.insert(case.plan.id.clone(), next);
                if next.is_none()
                    && outcome.map_or(case.aborted, |value| value.fails_run)
                {
                    self.success = false;
                }
                case.events.push(json!({
                "testCaseFinished": {
                "testCaseStartedId": case.started_id,

                "timestamp": timestamp(at)?,
                     "willBeRetried": next.is_some()}
                    }
                    ));
                self.flush_case(case)
            }
        }
    }

    fn skip_remaining(
        &mut self,
        case: &mut CaseRun,
        at: SystemTime,
    ) -> Result<(), Error> {
        while case.next_step < case.plan.steps.len() {
            // Unexecuted steps have no timings: use the observed cleanup/end
            // boundary and zero duration, before any real after-hook events.
            case.step_started = Some(at);
            self.step_started(case, at)?;
            self.step_finished(case, at, "SKIPPED", None)?;
        }
        Ok(())
    }

    fn insert_attachments(
        &self,
        case: &mut CaseRun,
        observed: &scenario_hooks::ObservedHooks,
    ) -> Result<(), Error> {
        let Some(store) = &self.attachment_store else { return Ok(()) };
        let records = store.take(
            &case.uri,
            case.plan.rule.as_ref(),
            &case.plan.scenario,
            case.retries.map_or(0, |retry| retry.current),
        );
        let mut pending: HashMap<String, Vec<Value>> = HashMap::new();
        for record in records {
            use crate::attachment::{Body, Target};
            let target = match record.target {
                Target::BeforeScenario if observed.initialization => {
                    scenario_hooks::initialization_step_id(case)
                }
                Target::BeforeScenario if case.hooks[0] => {
                    hook_step_id(case, 0)
                }
                Target::ScenarioHook(id) => {
                    scenario_hooks::callback_step_id(case, id)
                }
                Target::AfterScenario if case.hooks[1] => hook_step_id(case, 1),
                Target::Step(index) => case
                    .plan
                    .test_steps
                    .get(index)
                    .and_then(|step| step["id"].as_str())
                    .map(str::to_owned)
                    .ok_or(Error::InvalidEvent(
                        "attachment target step is absent",
                    ))?,
                _ => {
                    return Err(Error::InvalidEvent(
                        "attachment target hook is absent",
                    ));
                }
            };
            let (body, encoding) = match record.attachment.body {
                Body::Text(text) => (text, "IDENTITY"),
                Body::Bytes(bytes) => (
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                    "BASE64",
                ),
            };
            let mut attachment = json!({
                "body": body,
                 "contentEncoding": encoding,

                "mediaType": record.attachment.media_type,
                     "testCaseStartedId": case.started_id,

                "testStepId": target, "timestamp": timestamp(record.at)?});
            if let Some(name) = record.attachment.file_name {
                attachment["fileName"] = json!(name);
            }
            pending
                .entry(target)
                .or_default()
                .push(json!({"attachment": attachment}));
        }
        let mut events = Vec::new();
        for envelope in std::mem::take(&mut case.events) {
            if let Some(finished) = envelope.get("testStepFinished") {
                if let Some(id) = finished["testStepId"].as_str() {
                    if let Some(attachments) = pending.remove(id) {
                        events.extend(attachments);
                    }
                }
            }
            events.push(envelope);
        }
        if !pending.is_empty() {
            return Err(Error::InvalidEvent(
                "attachment target has no completed execution",
            ));
        }
        case.events = events;
        Ok(())
    }

    fn flush_case(&mut self, mut case: CaseRun) -> Result<(), Error> {
        if let Some(snapshot) = &self.support_code {
            for (native, planned) in
                case.plan.steps.iter().zip(&mut case.plan.test_steps)
            {
                let definitions = snapshot.matching_definitions(native);
                planned["stepDefinitionIds"] = json!(
                    definitions.iter().map(|d| d.id()).collect::<Vec<_>>()
                );
                if !definitions.is_empty()
                    && definitions.iter().all(|d| {
                        d.expression_metadata().is_some()
                            || d.regular_expression_metadata().is_some()
                    })
                {
                    let lists = definitions
                        .iter()
                        .map(|definition| {
                            let arguments = if let Some(metadata) =
                                definition.expression_metadata()
                            {
                                metadata
                                    .arguments(
                                        definition.regex(),
                                        &native.value,
                                    )
                                    .map_err(Error::Metadata)?
                                    .iter()
                                    .map(match_argument)
                                    .collect::<Vec<_>>()
                            } else {
                                let metadata = definition
                                    .regular_expression_metadata()
                                    .expect("presence checked above");
                                metadata
                                    .arguments(
                                        definition.regex(),
                                        &native.value,
                                    )
                                    .map_err(Error::Metadata)?
                                    .iter()
                                    .map(|group| {
                                        json!({
                                        "group": argument_group(group)}
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            };
                            Ok(json!({"stepMatchArguments": arguments}))
                        })
                        .collect::<Result<Vec<Value>, Error>>()?;
                    planned["stepMatchArgumentsLists"] = json!(lists);
                }
            }
        }
        let observed = self.observed_scenario_hooks(&mut case)?;
        self.insert_attachments(&mut case, &observed)?;
        let mut steps = Vec::new();
        if let Some(before) = &observed.steps[0] {
            steps.extend(before.iter().cloned());
        } else if case.hooks[0] {
            steps.push(
                json!({"id": hook_step_id(&case, 0), "hookId": hook_id(0)}),
            );
        }
        steps.extend(case.plan.test_steps.iter().cloned());
        if let Some(after) = &observed.steps[1] {
            steps.extend(after.iter().cloned());
        } else if case.hooks[1] {
            steps.push(
                json!({"id": hook_step_id(&case, 1), "hookId": hook_id(1)}),
            );
        }
        if let Some(previous) = self.emitted_cases.get(&case.plan.id) {
            if previous != &steps {
                return Err(Error::InvalidEvent(
                    "hook stages changed between retry attempts",
                ));
            }
        } else {
            for (index, present) in case.hooks.iter().enumerate() {
                if *present && !self.emitted_hooks[index] {
                    // Native events identify only the stage, not registered
                    // callbacks or their source. Empty reference is permitted
                    // by Messages 34.2.1; never invent a URI or line number.
                    self.emit(json!({"hook": {"id": hook_id(index),
                        "name": if index == 0 {
                             "Native before-scenario composite stage" }
                             else {
                             "Native after-scenario composite stage" }
                            ,

                        "type": if index == 0 {
                             "BEFORE_TEST_CASE" }
                             else {
                             "AFTER_TEST_CASE" }
                            ,

                        "sourceReference": {}}}))?;
                    self.emitted_hooks[index] = true;
                }
            }
            self.emit(json!({
                "testCase": {
                "id": case.plan.id,
                 "pickleId": case.plan.pickle_id,

                "testSteps": steps, "testRunStartedId": "run-0"}}))?;
            self.emitted_cases.insert(case.plan.id.clone(), steps);
        }
        for envelope in case.events {
            self.emit(envelope)?;
        }
        Ok(())
    }

    fn step_started(
        &mut self,
        case: &mut CaseRun,
        at: SystemTime,
    ) -> Result<(), Error> {
        case.events.push(json!({
        "testStepStarted": {
        "testCaseStartedId": case.started_id,

        "testStepId": case.plan.test_steps[case.next_step]["id"],
             "timestamp": timestamp(at)?}
            }
            ));
        Ok(())
    }

    fn step_finished(
        &mut self,
        case: &mut CaseRun,
        at: SystemTime,
        status: &str,
        message: Option<String>,
    ) -> Result<(), Error> {
        let started = case
            .step_started
            .take()
            .ok_or(Error::InvalidEvent("step finishes without starting"))?;
        let elapsed = at.duration_since(started).map_err(|_| {
            Error::InvalidEvent("step timestamp moved backwards")
        })?;
        let mut result =
            json!({"status": status, "duration": duration(elapsed)});
        if let Some(message) = message {
            result["message"] = Value::String(message);
        }
        case.events.push(
            json!({"testStepFinished": {"testCaseStartedId": case.started_id,
            "testStepId": case.plan.test_steps[case.next_step]["id"],
            "timestamp": timestamp(at)?, "testStepResult": result}}),
        );
        case.next_step += 1;
        Ok(())
    }
}

impl<W, O: io::Write> Writer<W> for Messages<O> {
    type Cli = cli::Empty;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        self.try_handle_event(event).unwrap_or_else(|error| panic!("{error}"));
    }
}

impl<O: io::Write> writer::NonTransforming for Messages<O> {}

fn match_argument(argument: &crate::support_code::MatchArgument) -> Value {
    json!({
    "group": argument_group(&argument.group),
     "parameterTypeName": argument.parameter_type_name}
    )
}

fn argument_group(group: &crate::support_code::ArgumentGroup) -> Value {
    let mut value = json!({});
    if let Some(start) = group.start {
        value["start"] = json!(start);
    }
    if let Some(text) = &group.value {
        value["value"] = json!(text);
    }
    if let Some(children) = &group.children {
        value["children"] =
            json!(children.iter().map(argument_group).collect::<Vec<_>>());
    }
    value
}

fn feature_uri(feature: &gherkin::Feature) -> Result<String, Error> {
    feature
        .path
        .as_ref()
        .and_then(|p| p.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            Error::Source("a UTF-8 source path is required".to_owned())
        })
}

fn validate_source(
    feature: &gherkin::Feature,
    source: &str,
) -> Result<(), Error> {
    if feature.keyword != "Feature"
        || source.lines().any(|line| {
            line.trim_start()
                .strip_prefix("# language:")
                .is_some_and(|language| language.trim() != "en")
        })
    {
        return Err(Error::Unsupported("only English source is supported"));
    }
    if feature.background.is_some()
        && feature.scenarios.iter().any(|s| s.steps.is_empty())
    {
        return Err(Error::Unsupported("empty scenarios with backgrounds"));
    }
    if !feature.rules.is_empty() {
        return Err(Error::Unsupported("rules"));
    }
    if !feature.tags.is_empty()
        || feature.scenarios.iter().any(|s| !s.tags.is_empty())
    {
        return Err(Error::Unsupported("tags"));
    }
    if feature.description.is_some()
        || feature.background.as_ref().is_some_and(|b| b.description.is_some())
        || feature.scenarios.iter().any(|s| s.description.is_some())
    {
        return Err(Error::Unsupported("descriptions"));
    }
    if feature.scenarios.iter().any(|s| {
        !s.examples.is_empty()
            || !matches!(s.keyword.as_str(), "Scenario" | "Example")
    }) {
        return Err(Error::Unsupported("scenario outlines"));
    }
    if feature
        .background
        .iter()
        .flat_map(|b| b.steps.iter())
        .chain(feature.scenarios.iter().flat_map(|s| s.steps.iter()))
        .any(|s| s.docstring.is_some() || s.table.is_some())
    {
        return Err(Error::Unsupported("step arguments"));
    }
    Ok(())
}

fn ast_id(uri: &str, kind: &str, position: gherkin::LineCol) -> String {
    format!("{uri}#ast-{kind}-{}-{}", position.line, position.col)
}

fn location(position: gherkin::LineCol) -> Value {
    json!({"line": position.line, "column": position.col})
}

fn ast_step(uri: &str, step: &gherkin::Step) -> Value {
    json!({
    "id": ast_id(uri,
     "step",
     step.position),
     "location": location(step.position),

    "keyword": step.keyword,
         "keywordType": step_keyword_type(step),
         "text": step.value}
        )
}

fn step_keyword_type(step: &gherkin::Step) -> &'static str {
    match step.keyword.trim() {
        "Given" => "Context",
        "When" => "Action",
        "Then" => "Outcome",
        "And" | "But" => "Conjunction",
        _ => "Unknown",
    }
}

fn check_case(
    case: &CaseRun,
    uri: &str,
    rule: Option<&gherkin::Rule>,
    scenario: &gherkin::Scenario,
) -> Result<(), Error> {
    if case.uri != uri
        || &case.plan.scenario != scenario
        || !same_rule(case.plan.rule.as_ref(), rule)
    {
        Err(Error::InvalidEvent("event belongs to another scenario"))
    } else {
        Ok(())
    }
}

fn same_rule(
    expected: Option<&gherkin::Rule>,
    actual: Option<&gherkin::Rule>,
) -> bool {
    crate::feature::same_rule_identity(expected, actual)
}

fn timestamp(at: SystemTime) -> Result<Value, Error> {
    at.duration_since(UNIX_EPOCH)
        .map(duration)
        .map_err(|_| Error::InvalidEvent("timestamp predates Unix epoch"))
}

fn duration(value: Duration) -> Value {
    json!({"seconds": value.as_secs(), "nanos": value.subsec_nanos()})
}

fn hook_id(index: usize) -> &'static str {
    if index == 0 {
        "native-hook-before-scenario"
    } else {
        "native-hook-after-scenario"
    }
}

fn hook_step_id(case: &CaseRun, index: usize) -> String {
    format!("{}-{}", case.plan.id, hook_id(index))
}

fn outcome_status(status: crate::execution::OutcomeStatus) -> &'static str {
    use crate::execution::OutcomeStatus::*;
    match status {
        Passed => "PASSED",
        Skipped => "SKIPPED",
        Pending => "PENDING",
        Undefined => "UNDEFINED",
        Ambiguous => "AMBIGUOUS",
        Failed => "FAILED",
    }
}
