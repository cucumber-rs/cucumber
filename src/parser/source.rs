//! Immutable source snapshots shared between parsing and reporting.

use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, RwLock}};

use super::basic::{DialectCatalog, SourceFormat};

/// Exact input and raw native AST captured before outline expansion.
#[derive(Debug)]
pub struct SourceSnapshot {
    source: Arc<str>,
    feature: gherkin::Feature,
    default_language: String,
    dialect_catalog: DialectCatalog,
    docstring_semantics: gherkin::DocStringSemantics,
    example_semantics: crate::feature::ExampleSemantics,
    table_semantics: gherkin::TableSemantics,
    lexical_semantics: gherkin::LexicalSemantics,
    source_format: SourceFormat,
    markdown: Option<Arc<gherkin::markdown::MarkdownDocument>>,
}

impl SourceSnapshot {
    /// Returns original source locations in this snapshot's expansion order.
    ///
    /// Each record identifies one execution candidate before filtering. Duplicate
    /// rows have distinct ordinals and actual row lines; synthetic native expanded
    /// positions are never consulted. No filesystem access or Messages is needed.
    ///
    /// # Errors
    /// Reports a mismatch between captured table spans and validated native rows.
    pub fn scenario_origins(&self) -> Result<Vec<ScenarioOrigin>, SourceOriginError> {
        let mut origins = Vec::new();
        self.collect_origins(&self.feature.scenarios, None, &mut origins)?;
        for (index, rule) in self.feature.rules.iter().enumerate() {
            self.collect_origins(&rule.scenarios, Some(index), &mut origins)?;
        }
        Ok(origins)
    }

    fn collect_origins(&self, scenarios: &[gherkin::Scenario], rule_index: Option<usize>, origins: &mut Vec<ScenarioOrigin>) -> Result<(), SourceOriginError> {
        let markdown = self.markdown().and_then(|document| document.feature.as_ref()).map(|feature| {
            rule_index.map_or(feature.scenarios.as_slice(), |index| feature.rules[index].scenarios.as_slice())
        });
        let mut expanded_index = 0;
        for (raw_index, scenario) in scenarios.iter().enumerate() {
            if scenario.examples.is_empty() {
                origins.push(ScenarioOrigin { rule_index, scenario_index: expanded_index,
                    scenario_line: scenario.position.line, example_line: None });
                expanded_index += 1;
                continue;
            }
            for (example_index, examples) in scenario.examples.iter().enumerate() {
                let Some(table) = &examples.table else { continue; };
                let lines = if let Some(markdown) = markdown {
                    markdown[raw_index].examples[example_index].rows.iter().map(|row| row.position.line).collect::<Vec<_>>()
                } else { self.table_lines(table)? };
                if lines.len() != table.rows.len() {
                    return Err(SourceOriginError("source Examples row count differs from captured native table".into()));
                }
                for line in lines.into_iter().skip(1) {
                    origins.push(ScenarioOrigin { rule_index, scenario_index: expanded_index,
                        scenario_line: scenario.position.line, example_line: Some(line) });
                    expanded_index += 1;
                }
            }
        }
        Ok(())
    }

    fn table_lines(&self, table: &gherkin::Table) -> Result<Vec<usize>, SourceOriginError> {
        let text = self.source.get(table.span.start..table.span.end)
            .ok_or_else(|| SourceOriginError("captured Examples table has an invalid source span".into()))?;
        let first = self.source[..table.span.start].bytes().filter(|byte| *byte == b'\n').count() + 1;
        Ok(text.split_inclusive('\n').enumerate().filter_map(|(index, line)| {
            line.trim_start_matches(|character| self.lexical_semantics.is_padding(character)).starts_with('|').then_some(first + index)
        }).collect())
    }

    /// Returns the classic token policy captured for this immutable generation.
    #[must_use]
    pub const fn lexical_semantics(&self) -> gherkin::LexicalSemantics { self.lexical_semantics }

    /// Returns the resolved source syntax captured with this generation.
    #[must_use]
    pub const fn source_format(&self) -> SourceFormat { self.source_format }

    /// Returns the typed Markdown source document, when that syntax was selected.
    #[must_use]
    pub fn markdown(&self) -> Option<&gherkin::markdown::MarkdownDocument> { self.markdown.as_deref() }

    /// Returns the exact UTF-8 source, including its original line endings.
    #[must_use]
    pub fn source(&self) -> &str { &self.source }

    /// Returns the raw parsed feature before examples or filters transform it.
    #[must_use]
    pub fn feature(&self) -> &gherkin::Feature { &self.feature }

    /// Returns the configured fallback dialect; a source directive overrides it.
    #[must_use]
    pub fn default_language(&self) -> &str { &self.default_language }

    /// Returns the table row sizing policy used when parsing this snapshot.
    #[must_use]
    pub const fn table_semantics(&self) -> gherkin::TableSemantics { self.table_semantics }

    /// Returns the outline interpolation policy used for runtime candidates.
    #[must_use]
    pub const fn example_semantics(&self) -> crate::feature::ExampleSemantics { self.example_semantics }

    /// Returns the vocabulary version used when parsing this snapshot.
    #[must_use]
    pub const fn dialect_catalog(&self) -> DialectCatalog { self.dialect_catalog }

    /// Returns the content policy used for this snapshot's native doc strings.
    #[must_use]
    pub const fn docstring_semantics(&self) -> gherkin::DocStringSemantics {
        self.docstring_semantics
    }
}

/// A validated featureless document captured by explicit latest parser mode.
///
/// Retaining this record preserves its generation after replacement or failure.
#[derive(Debug)]
pub struct EmptyDocumentSnapshot {
    lexical_semantics: gherkin::LexicalSemantics,
    path: PathBuf,
    source: Arc<str>,
    default_language: String,
    source_format: SourceFormat,
    markdown: Option<Arc<gherkin::markdown::MarkdownDocument>>,
}

impl EmptyDocumentSnapshot {
    /// Returns the classic lexical policy used to validate these exact bytes.
    #[must_use]
    pub const fn lexical_semantics(&self) -> gherkin::LexicalSemantics { self.lexical_semantics }

    /// Returns the resolved source syntax captured with this generation.
    #[must_use]
    pub const fn source_format(&self) -> SourceFormat { self.source_format }

    /// Returns the typed Markdown source document, when that syntax was selected.
    #[must_use]
    pub fn markdown(&self) -> Option<&gherkin::markdown::MarkdownDocument> { self.markdown.as_deref() }

    /// Returns the exact parser source identity without another filesystem read.
    #[must_use]
    pub fn path(&self) -> &Path { &self.path }

    /// Returns exact captured UTF-8 bytes, including original line endings.
    #[must_use]
    pub fn source(&self) -> &str { &self.source }

    /// Returns the configured fallback dialect used for validation.
    #[must_use]
    pub fn default_language(&self) -> &str { &self.default_language }
}

/// Per-run source store for a parser and its reporting consumers.
///
/// Clones share the current entries. Each entry is immutable: retaining an Arc
/// keeps that generation even if a later parse replaces the same URI. Use a
/// fresh store for independent runs, particularly concurrent runs with the same
/// source paths. No filesystem access occurs when reading a snapshot.
#[derive(Clone, Debug, Default)]
pub struct SourceStore(Arc<RwLock<CapturedSources>>);

#[derive(Debug, Default)]
struct CapturedSources {
    snapshots: HashMap<PathBuf, Arc<SourceSnapshot>>,
    inputs: HashMap<PathBuf, Arc<str>>,
    input_formats: HashMap<PathBuf, SourceFormat>,
    empty_documents: HashMap<PathBuf, Arc<EmptyDocumentSnapshot>>,
}

impl SourceStore {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Retrieves the latest snapshot for the exact path used by the parser.
    #[must_use]
    pub fn get(&self, path: &Path) -> Option<Arc<SourceSnapshot>> {
        self.0.read().expect("source store lock poisoned").snapshots.get(path).cloned()
    }

    /// Retrieves the current validated featureless record for this parser path.
    #[must_use]
    pub fn empty_document(&self, path: &Path) -> Option<Arc<EmptyDocumentSnapshot>> {
        self.0.read().expect("source store lock poisoned").empty_documents.get(path).cloned()
    }

    /// Returns current featureless records in deterministic source-path order.
    #[must_use]
    pub fn empty_documents(&self) -> Vec<Arc<EmptyDocumentSnapshot>> {
        let mut documents = self.0.read().expect("source store lock poisoned")
            .empty_documents.values().cloned().collect::<Vec<_>>();
        documents.sort_by(|left, right| left.path.cmp(&right.path));
        documents
    }

    /// Retrieves exact input even when parsing or example expansion failed.
    /// A failed read clears the current input; retained snapshots stay immutable.
    #[must_use]
    pub fn input(&self, path: &Path) -> Option<Arc<str>> {
        self.0.read().expect("source store lock poisoned").inputs.get(path).cloned()
    }

    /// Returns the syntax of current captured bytes, including invalid input.
    /// Failed reads and unresolved Auto paths have no current format.
    #[must_use]
    pub fn input_format(&self, path: &Path) -> Option<SourceFormat> {
        self.0.read().expect("source store lock poisoned").input_formats.get(path).copied()
    }

    pub(crate) fn clear(&self, path: &Path) {
        let mut captured = self.0.write().expect("source store lock poisoned");
        captured.snapshots.remove(path);
        captured.empty_documents.remove(path);
        captured.inputs.remove(path);
        captured.input_formats.remove(path);
    }

    pub(crate) fn capture_input(&self, path: &Path, source: &str, format: SourceFormat) {
        let mut captured = self.0.write().expect("source store lock poisoned");
        captured.snapshots.remove(path);
        captured.empty_documents.remove(path);
        captured.inputs.insert(path.to_path_buf(), Arc::from(source));
        captured.input_formats.insert(path.to_path_buf(), format);
    }

    pub(crate) fn capture_empty(&self, path: PathBuf, source: String, default_language: String, lexical_semantics: gherkin::LexicalSemantics) {
        let mut captured = self.0.write().expect("source store lock poisoned");
        captured.snapshots.remove(&path);
        let source = captured.inputs.entry(path.clone()).or_insert_with(|| Arc::from(source)).clone();
        captured.empty_documents.insert(path.clone(), Arc::new(EmptyDocumentSnapshot { path, source, default_language, lexical_semantics, source_format: SourceFormat::Plain, markdown: None }));
    }

    pub(crate) fn capture_markdown(&self, path: PathBuf, source: String,
        document: gherkin::markdown::MarkdownDocument, default_language: String)
    {
        let mut captured = self.0.write().expect("source store lock poisoned");
        let source = captured.inputs.entry(path.clone()).or_insert_with(|| Arc::from(source)).clone();
        captured.input_formats.insert(path.clone(), SourceFormat::Markdown);
        captured.snapshots.remove(&path);
        captured.empty_documents.remove(&path);
        let document = Arc::new(document);
        if let Some(feature) = document.to_feature_at(&path) {
            captured.snapshots.insert(path, Arc::new(SourceSnapshot { source, feature, default_language,
                dialect_catalog: DialectCatalog::Gherkin43,
                docstring_semantics: gherkin::DocStringSemantics::Gherkin43,
                example_semantics: crate::feature::ExampleSemantics::Gherkin43,
                table_semantics: gherkin::TableSemantics::Gherkin43,
                lexical_semantics: gherkin::LexicalSemantics::Gherkin43,
                source_format: SourceFormat::Markdown, markdown: Some(document) }));
        } else {
            captured.empty_documents.insert(path.clone(), Arc::new(EmptyDocumentSnapshot {
                path, source, default_language, lexical_semantics: gherkin::LexicalSemantics::Gherkin43, source_format: SourceFormat::Markdown,
                markdown: Some(document) }));
        }
    }

    pub(crate) fn capture(&self, path: PathBuf, source: String,
        feature: gherkin::Feature, default_language: String, dialect_catalog: DialectCatalog,
        docstring_semantics: gherkin::DocStringSemantics, example_semantics: crate::feature::ExampleSemantics,
        table_semantics: gherkin::TableSemantics, lexical_semantics: gherkin::LexicalSemantics)
    {
        let mut captured = self.0.write().expect("source store lock poisoned");
        let source = captured.inputs.entry(path.clone()).or_insert_with(|| Arc::from(source)).clone();
        captured.empty_documents.remove(&path);
        captured.snapshots.insert(path,
            Arc::new(SourceSnapshot { source, feature, default_language, dialect_catalog, docstring_semantics, example_semantics, table_semantics, lexical_semantics, source_format: SourceFormat::Plain, markdown: None }));
    }
}

/// Original source locations for one expanded scenario candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScenarioOrigin {
    rule_index: Option<usize>,
    scenario_index: usize,
    scenario_line: usize,
    example_line: Option<usize>,
}
impl ScenarioOrigin {
    /// Zero-based Rule index, or None for a feature-level scenario.
    #[must_use]
    pub const fn rule_index(&self) -> Option<usize> { self.rule_index }
    /// Zero-based expanded ordinal within this feature or Rule before filtering.
    #[must_use]
    pub const fn scenario_index(&self) -> usize { self.scenario_index }
    /// Actual one-based scenario or outline declaration line.
    #[must_use]
    pub const fn scenario_line(&self) -> usize { self.scenario_line }
    /// Actual one-based Examples body-row line, absent for a plain scenario.
    #[must_use]
    pub const fn example_line(&self) -> Option<usize> { self.example_line }
}

/// A captured source cannot be aligned with its native Examples rows.
#[derive(Clone, Debug)]
pub struct SourceOriginError(String);
impl std::fmt::Display for SourceOriginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) }
}
impl std::error::Error for SourceOriginError {}
