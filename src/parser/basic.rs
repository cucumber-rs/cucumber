// Copyright (c) 2018-2026  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Default [`Parser`] implementation.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    str::FromStr,
    vec,
};

use derive_more::with_trait::{Display, Error};
use futures::stream;
use gherkin::GherkinEnv;
use globwalk::{GlobWalker, GlobWalkerBuilder};
use itertools::Itertools as _;

use super::{Error as ParseError, Parser, SourceStore};
use crate::feature::Ext as _;

/// CLI options of a [`Basic`] [`Parser`].
#[derive(Clone, Debug, Default, clap::Args)]
#[group(skip)]
pub struct Cli {
    /// Glob pattern to look for feature files with. If not specified, looks for
    /// `*.feature` files in the path configured in the test runner.
    #[arg(
        id = "input",
        long = "input",
        short = 'i',
        value_name = "glob",
        global = true
    )]
    pub features: Option<Walker>,
}

/// Versioned keyword catalogue used to interpret Gherkin dialects.
///
/// This selects dialect vocabulary only, not complete grammar or runtime
/// conformance. Doc-string content semantics are configured independently.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DialectCatalog {
    /// Original gherkin 0.16 vocabulary, preserving existing applications.
    #[default]
    Legacy,
    /// Official Gherkin 43.0.0 vocabulary and longest-keyword preference.
    Gherkin43,
}

/// Explicit source syntax and optional filename-based dispatch.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SourceFormat {
    /// Classic Gherkin on any supplied path; historical default.
    #[default]
    Plain,
    /// Native Gherkin43 Markdown on any supplied path.
    Markdown,
    /// Dispatch `.feature` to Plain and `.md` to Markdown; reject other extensions.
    Auto,
}

impl SourceFormat {
    fn resolve(self, path: &Path) -> Option<Self> {
        match self {
            Self::Auto => match path.extension().and_then(|s| s.to_str()) {
                Some(ext) if ext.eq_ignore_ascii_case("feature") => Some(Self::Plain),
                Some(ext) if ext.eq_ignore_ascii_case("md") => Some(Self::Markdown),
                _ => None,
            },
            format => Some(format),
        }
    }

    /// Returns the standard media type of a resolved syntax; Auto is unresolved.
    #[must_use]
    pub const fn media_type(self) -> Option<&'static str> {
        match self {
            Self::Plain => Some("text/x.cucumber.gherkin+plain"),
            Self::Markdown => Some("text/x.cucumber.gherkin+markdown"),
            Self::Auto => None,
        }
    }
}

/// Explicit ordered source selection, independent of the legacy single input.
///
/// Each expansion is sorted; the complete selection preserves entry order and
/// deduplicates canonical existing files at their first occurrence. Literal
/// missing paths report reading errors; unmatched globs select nothing.
#[derive(Clone, Debug, Default)]
pub struct SourcePaths(Vec<SourcePath>);
#[derive(Clone, Debug)]
enum SourcePath { Path(PathBuf), Glob(Walker) }
impl SourcePaths {
    /// Starts an empty explicit plan (no implicit default discovery).
    #[must_use]
    pub fn new() -> Self { Self::default() }
    /// Appends an exact file or recursively discovered directory.
    #[must_use]
    pub fn path(mut self, path: impl Into<PathBuf>) -> Self {
        self.0.push(SourcePath::Path(path.into())); self
    }
    /// Appends a validated globwalk pattern. Matches are sorted within this entry.
    ///
    /// # Errors
    /// Returns the glob parser error for an invalid pattern.
    pub fn glob(mut self, pattern: &str) -> Result<Self, globwalk::GlobError> {
        self.0.push(SourcePath::Glob(pattern.parse()?)); Ok(self)
    }
}

/// Default [`Parser`].
///
/// As there is no async runtime-agnostic way to interact with IO, this
/// [`Parser`] is blocking.
#[derive(Clone, Debug, Default)]
pub struct Basic {
    /// Optional custom language of [`gherkin`] keywords.
    ///
    /// Default is English.
    language: Option<Cow<'static, str>>,

    /// Optional exact-source capture for reporting before expansion.
    sources: Option<SourceStore>,

    /// Explicit dialect vocabulary version.
    dialect_catalog: DialectCatalog,

    /// Explicit doc-string content policy; legacy content is the default.
    docstring_semantics: gherkin::DocStringSemantics,

    /// Outline interpolation compatibility policy.
    example_semantics: crate::feature::ExampleSemantics,

    /// Explicit table row sizing; legacy truncation is the default.
    table_semantics: gherkin::TableSemantics,

    /// Classic lexical compatibility, independent of vocabulary.
    lexical_semantics: gherkin::LexicalSemantics,

    /// Explicit document mode accepts validated sources without a Feature.
    allow_empty_documents: bool,

    /// Optional source-aware selection after native example expansion.
    line_selectors: Option<crate::filter::LineSelectors>,

    /// Source syntax, independent of classic compatibility policies.
    source_format: SourceFormat,

    /// Opt-in complete ordered source plan.
    source_paths: Option<SourcePaths>,
}

impl<I: AsRef<Path>> Parser<I> for Basic {
    type Cli = Cli;

    type Output =
        stream::Iter<vec::IntoIter<Result<gherkin::Feature, ParseError>>>;

    fn parse(self, input: I, cli: Self::Cli) -> Self::Output {
        let walk = |walker: GlobWalker| {
            walker
                .filter_map(Result::ok)
                .sorted_by(|l, r| Ord::cmp(l.path(), r.path()))
                .map(|file| {
                    let env = self.environment();
                    self.parse_path(file.path(), env)
                })
                .collect::<Vec<_>>()
        };

        let get_features_path = || {
            let path = input.as_ref();
            path.canonicalize()
                .or_else(|_| {
                    let buf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                        path.strip_prefix("/")
                            .or_else(|_| path.strip_prefix("./"))
                            .unwrap_or(path),
                    );
                    buf.as_path().canonicalize()
                })
                .map_err(|e| gherkin::ParseFileError::Reading {
                    path: path.to_path_buf(),
                    source: e,
                })
        };

        let features = || {
            let features = if let Some(paths) = &self.source_paths {
                self.discover_paths(paths).into_iter().map(|path|
                    path.and_then(|path| self.parse_path(path, self.environment()))
                ).collect()
            } else if let Some(walker) = cli.features {
                walk(globwalk::glob(walker.0).unwrap_or_else(|e| {
                    unreachable!("invalid glob pattern: {e}")
                }))
            } else {
                let feats_path = match get_features_path() {
                    Ok(p) => p,
                    Err(e) => {
                        if let Some(store) = &self.sources { store.clear(input.as_ref()); }
                        return vec![Err(e.into())];
                    },
                };

                if feats_path.is_file() {
                    let env = self.environment();
                    vec![self.parse_path(feats_path, env)]
                } else {
                    let patterns: &[&str] = match self.source_format {
                        SourceFormat::Plain => &["*.feature"],
                        SourceFormat::Markdown => &["*.feature.md"],
                        SourceFormat::Auto => &["*.feature", "*.feature.md"],
                    };
                    let w = GlobWalkerBuilder::from_patterns(feats_path, patterns)
                        .case_insensitive(true)
                        .build()
                        .unwrap_or_else(|e| {
                            unreachable!("`GlobWalkerBuilder` panicked: {e}")
                        });
                    walk(w)
                }
            };

            features
                .into_iter()
                .filter_map(|f| match f {
                    Ok(Some(f)) => {
                        let semantics = if f.path.as_deref().and_then(|path| self.source_format.resolve(path)) == Some(SourceFormat::Markdown) {
                            crate::feature::ExampleSemantics::Gherkin43
                        } else { self.example_semantics };
                        Some(f.expand_examples_with(semantics).map_err(ParseError::from)
                            .and_then(|feature| self.select_lines(feature)))
                    },
                    Ok(None) => None,
                    Err(e) => Some(Err(e.into())),
                })
                .collect()
        };

        stream::iter(features())
    }
}

impl Basic {
    /// Creates a new [`Basic`] [`Parser`].
    #[must_use]
    pub const fn new() -> Self {
        Self { language: None, sources: None, dialect_catalog: DialectCatalog::Legacy,
            docstring_semantics: gherkin::DocStringSemantics::Legacy,
            example_semantics: crate::feature::ExampleSemantics::Legacy,
            table_semantics: gherkin::TableSemantics::Legacy, lexical_semantics: gherkin::LexicalSemantics::Legacy, allow_empty_documents: false, line_selectors: None, source_format: SourceFormat::Plain, source_paths: None }
    }

    /// Uses an ordered complete plan instead of the run input or CLI glob.
    /// Defaults remain unchanged unless this builder is called.
    #[must_use]
    pub fn with_source_paths(mut self, paths: SourcePaths) -> Self {
        self.source_paths = Some(paths); self
    }

    fn discover_paths(&self, paths: &SourcePaths)
        -> Vec<Result<PathBuf, gherkin::ParseFileError>>
    {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let reading = |path: PathBuf, source: std::io::Error| {
            if let Some(store) = &self.sources { store.clear(&path); }
            gherkin::ParseFileError::Reading { path, source }
        };
        for entry in &paths.0 {
            let (base, walker) = match entry {
                SourcePath::Path(path) => match path.canonicalize() {
                    Ok(path) if path.is_file() => {
                        if seen.insert(path.clone()) { result.push(Ok(path)); }
                        continue;
                    },
                    Ok(path) => {
                        let patterns: &[&str] = match self.source_format {
                            SourceFormat::Plain => &["*.feature"],
                            SourceFormat::Markdown => &["*.feature.md"],
                            SourceFormat::Auto => &["*.feature", "*.feature.md"],
                        };
                        let walker = GlobWalkerBuilder::from_patterns(&path, patterns)
                            .case_insensitive(true).build();
                        (path, walker)
                    },
                    Err(error) => { result.push(Err(reading(path.clone(), error))); continue; },
                },
                SourcePath::Glob(pattern) => (PathBuf::from(&pattern.0), globwalk::glob(&pattern.0)),
            };
            let walker = match walker {
                Ok(walker) => walker,
                Err(error) => { result.push(Err(reading(base, std::io::Error::other(error.to_string())))); continue; },
            };
            let mut matches = Vec::new();
            for item in walker {
                match item {
                    Ok(item) if item.file_type().is_file() => match item.path().canonicalize() {
                        Ok(path) => matches.push(path),
                        Err(error) => result.push(Err(reading(item.path().to_path_buf(), error))),
                    },
                    Ok(_) => {},
                    Err(error) => result.push(Err(reading(base.clone(), std::io::Error::other(error.to_string())))),
                }
            }
            matches.sort_by(|left, right| match (left.to_str(), right.to_str()) {
                (Some(left), Some(right)) => left.encode_utf16().cmp(right.encode_utf16()),
                _ => left.cmp(right),
            });
            for path in matches { if seen.insert(path.clone()) { result.push(Ok(path)); } }
        }
        result
    }

    /// Captures exact source and raw AST before expansion for reporting.
    #[must_use]
    pub fn with_source_store(mut self, sources: SourceStore) -> Self {
        self.sources = Some(sources);
        self
    }

    /// Selects discovered executions by original scenario/Examples body lines.
    ///
    /// Source capture is enabled internally if not already configured. Captured
    /// documents remain complete; only expanded runtime candidates are filtered.
    #[must_use]
    pub fn with_line_selectors(mut self, selectors: crate::filter::LineSelectors) -> Self {
        self.line_selectors = Some(selectors);
        self.sources.get_or_insert_with(SourceStore::new);
        self
    }

    fn select_lines(&self, mut feature: gherkin::Feature) -> Result<gherkin::Feature, ParseError> {
        let Some(selectors) = &self.line_selectors else { return Ok(feature); };
        let path = feature.path.clone().unwrap_or_default();
        let position = feature.position;
        let error = |message: String| ParseError::from(gherkin::ParseFileError::Parsing {
            path: path.clone(), error: None,
            source: gherkin::ParseError::with_message(position, message),
        });
        let snapshot = self.sources.as_ref().and_then(|store| store.get(&path))
            .ok_or_else(|| error("line selection requires this parser's captured source generation".into()))?;
        let origins = snapshot.scenario_origins().map_err(|failure| error(failure.to_string()))?;
        let select = |scenarios: &mut Vec<gherkin::Scenario>, rule_index: Option<usize>| -> Result<(), ParseError> {
            let records = origins.iter().filter(|origin| origin.rule_index() == rule_index).collect::<Vec<_>>();
            if records.len() != scenarios.len() {
                return Err(error("captured source origins differ from native expansion count".into()));
            }
            let keep = records.iter().map(|origin| selectors.matches(&path, origin)
                .map_err(|failure| error(failure.to_string()))).collect::<Result<Vec<_>, _>>()?;
            let mut index = 0;
            scenarios.retain(|_| { let selected = keep[index]; index += 1; selected });
            Ok(())
        };
        select(&mut feature.scenarios, None)?;
        for (index, rule) in feature.rules.iter_mut().enumerate() {
            select(&mut rule.scenarios, Some(index))?;
        }
        Ok(feature)
    }

    /// Selects all currently adopted Gherkin 43 source policies explicitly.
    ///
    /// This enables the latest dialect catalogue, normalized doc strings,
    /// official table rows/outline interpolation and valid featureless documents.
    /// Separate policy builders remain available; this is not a blanket claim of
    /// arbitrary grammar or runtime conformance. Legacy remains the default.
    ///
    /// # Errors
    /// Rejects an existing fallback language absent from the current catalogue.
    pub fn with_gherkin43(self) -> Result<Self, UnsupportedLanguageError> {
        let mut parser = self.with_dialect_catalog(DialectCatalog::Gherkin43)?;
        parser.docstring_semantics = gherkin::DocStringSemantics::Gherkin43;
        parser.table_semantics = gherkin::TableSemantics::Gherkin43;
        parser.lexical_semantics = gherkin::LexicalSemantics::Gherkin43;
        parser.example_semantics = crate::feature::ExampleSemantics::Gherkin43;
        parser.allow_empty_documents = true;
        Ok(parser)
    }

    /// Selects explicit syntax without changing default classic parser policies.
    ///
    /// Directory discovery uses `*.feature` for Plain, `*.feature.md` for
    /// Markdown, and both for Auto. Explicit paths/globs remain caller-selected.
    /// Markdown always uses its official43 source semantics and global dialect.
    #[must_use]
    pub fn with_source_format(mut self, format: SourceFormat) -> Self {
        self.source_format = format;
        self
    }

    fn parse_path<P: AsRef<Path>>(&self, path: P, env: GherkinEnv)
        -> Result<Option<gherkin::Feature>, gherkin::ParseFileError>
    {
        let path = path.as_ref();
        let format = self.source_format.resolve(path).ok_or_else(|| {
            if let Some(store) = &self.sources { store.clear(path); }
            gherkin::ParseFileError::Parsing { path: path.to_path_buf(), error: None,
                source: gherkin::ParseError::with_message(gherkin::LineCol { line: 1, col: 1 },
                    "Auto source format requires a .feature or .md extension") }
        })?;
        if format == SourceFormat::Plain && !self.allow_empty_documents && self.sources.is_none() {
            return gherkin::Feature::parse_path(path, env).map(Some);
        }
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(source) => {
                if let Some(store) = &self.sources { store.clear(path); }
                return Err(gherkin::ParseFileError::Reading { path: path.to_path_buf(), source });
            }
        };
        if let Some(store) = &self.sources { store.capture_input(path, &source, format); }
        if format == SourceFormat::Markdown {
            let language = self.language.as_deref().unwrap_or("en");
            let document = gherkin::markdown::MarkdownDocument::parse(&source, language)
                .map_err(|error| gherkin::ParseFileError::Parsing { path: path.to_path_buf(), error: None,
                    source: gherkin::ParseError::with_message(error.position, error.message) })?;
            let feature = document.to_feature_at(path);
            if let Some(store) = &self.sources {
                store.capture_markdown(path.to_path_buf(), source, document, language.to_owned());
            }
            return Ok(feature);
        }
        let feature = if self.allow_empty_documents {
            gherkin::Document::parse_file_source(path, &source, env)?.feature
        } else {
            Some(gherkin::Feature::parse_file_source(path, &source, env)?)
        };
        if let Some(store) = &self.sources {
            let language = self.language.as_deref().unwrap_or("en").to_owned();
            if let Some(feature) = &feature {
                store.capture(path.to_path_buf(), source, feature.clone(), language,
                    self.dialect_catalog, self.docstring_semantics, self.example_semantics, self.table_semantics, self.lexical_semantics);
            } else {
                store.capture_empty(path.to_path_buf(), source, language, self.lexical_semantics);
            }
        }
        Ok(feature)
    }

    /// Selects classic token padding, value trimming and column semantics.
    #[must_use]
    pub fn with_lexical_semantics(mut self, semantics: gherkin::LexicalSemantics) -> Self {
        self.lexical_semantics = semantics;
        self
    }

    /// Selects table row sizing independently of other parser policies.
    ///
    /// Legacy preserves truncation of later rows longer than the first row.
    /// Gherkin43 rejects both shorter and longer step and Examples rows.
    #[must_use]
    pub fn with_table_semantics(mut self, semantics: gherkin::TableSemantics) -> Self {
        self.table_semantics = semantics;
        self
    }

    /// Selects Scenario Outline interpolation independently of parser vocabulary.
    #[must_use]
    pub fn with_example_semantics(mut self, semantics: crate::feature::ExampleSemantics) -> Self {
        self.example_semantics = semantics;
        self
    }

    /// Selects the doc-string content delivered to native steps and snapshots.
    ///
    /// `Legacy` remains the default. `Gherkin43` removes delimiter/header lines,
    /// dedents relative to the opening delimiter, and unescapes its separator.
    /// This policy is independent of dialect vocabulary selection.
    #[must_use]
    pub fn with_docstring_semantics(mut self, semantics: gherkin::DocStringSemantics) -> Self {
        self.docstring_semantics = semantics;
        self
    }

    /// Selects an explicit dialect vocabulary while preserving other settings.
    ///
    /// Select the catalogue before configuring a newly supported language.
    ///
    /// # Errors
    /// Returns an error if the configured language is absent from the catalogue.
    pub fn with_dialect_catalog(mut self, catalog: DialectCatalog)
        -> Result<Self, UnsupportedLanguageError>
    {
        let language = self.language.as_deref().unwrap_or("en");
        if !catalog.supports(language) {
            return Err(UnsupportedLanguageError(language.to_owned().into()));
        }
        self.dialect_catalog = catalog;
        Ok(self)
    }

    fn environment(&self) -> GherkinEnv {
        let language = self.language.as_deref().unwrap_or("en");
        match self.dialect_catalog {
            DialectCatalog::Legacy => GherkinEnv::new(language),
            DialectCatalog::Gherkin43 => GherkinEnv::new_latest(language),
        }.unwrap_or_else(|_| unreachable!("parser builder validates dialect and catalogue"))
            .with_docstring_semantics(self.docstring_semantics)
            .with_table_semantics(self.table_semantics)
            .with_lexical_semantics(self.lexical_semantics)
    }

    /// Sets the provided language to parse [`gherkin`] files with instead of
    /// the default one (English).
    ///
    /// # Errors
    ///
    /// If the provided language isn't supported.
    pub fn language(
        mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<Self, UnsupportedLanguageError> {
        let name = name.into();
        if !self.dialect_catalog.supports(&name) {
            return Err(UnsupportedLanguageError(name));
        }
        self.language = Some(name);
        Ok(self)
    }
}

impl DialectCatalog {
    fn supports(self, language: &str) -> bool {
        match self {
            Self::Legacy => gherkin::is_language_supported(language),
            Self::Gherkin43 => gherkin::is_latest_language_supported(language),
        }
    }
}

/// Error of [`gherkin`] not supporting keywords in some language.
#[derive(Clone, Debug, Display, Error)]
#[display("Language {_0} isn't supported")]
pub struct UnsupportedLanguageError(
    #[error(not(source))] pub Cow<'static, str>,
);

/// Wrapper over [`GlobWalker`] implementing a [`FromStr`].
#[derive(Clone, Debug)]
pub struct Walker(String);

impl FromStr for Walker {
    type Err = globwalk::GlobError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        globwalk::glob(s).map(|_| Self(s.to_owned()))
    }
}
