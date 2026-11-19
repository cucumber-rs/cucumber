// Copyright (c) 2018-2026  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Explicit composable scenario selection.
//!
//! Names use Rust regular expressions and are joined by OR. Tag expressions
//! use [`crate::tag::Expression`] and are joined by AND. Installing these
//! options
//! with [`crate::Cucumber::with_filter_options`] also requires the caller's
//! `filter_run` predicate to accept the scenario. Without explicit options,
//! historical CLI override behavior remains unchanged.

use gherkin::tagexpr::TagOperation;
use regex::Regex;

use crate::tag::{Expression, Ext as _};

/// Repeatable name and tag selectors for explicit composable filtering.
///
/// An empty name group accepts every name; an empty tag group accepts all tags.
/// Native parser expansion supplies expanded names and Examples tags before
/// selection. Feature and Rule tags are inherited during matching.
#[derive(Clone, Debug, Default)]
pub struct FilterOptions {
    names: Vec<Regex>,
    tags: Vec<Expression>,
}

impl FilterOptions {
    /// Creates unrestricted typed selectors.
    #[must_use]
    pub const fn new() -> Self {
        Self { names: Vec::new(), tags: Vec::new() }
    }

    /// Adds an alternative name expression (OR with other name expressions).
    #[must_use]
    pub fn name(mut self, expression: Regex) -> Self {
        self.names.push(expression);
        self
    }

    /// Adds a required tag expression (AND with other tag expressions).
    #[must_use]
    pub fn tag(mut self, expression: Expression) -> Self {
        self.tags.push(expression);
        self
    }

    /// Matches this selector against a scenario and its inherited tags.
    #[must_use]
    pub fn matches(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
    ) -> bool {
        self.matches_with_cli(feature, rule, scenario, None, None)
    }

    pub(crate) fn matches_with_cli(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        name: Option<&Regex>,
        tags: Option<&TagOperation>,
    ) -> bool {
        let names_match = (self.names.is_empty() && name.is_none())
            || self
                .names
                .iter()
                .chain(name)
                .any(|name| name.is_match(&scenario.name));
        names_match
            && self.tags.iter().all(|tag| tag.matches(feature, rule, scenario))
            && tags.is_none_or(|tags| {
                tags.eval(
                    feature
                        .tags
                        .iter()
                        .chain(rule.into_iter().flat_map(|rule| &rule.tags))
                        .chain(&scenario.tags),
                )
            })
    }
}

/// Explicit per-source line selectors, independent of Messages reporting.
///
/// Only scenario declarations and Examples body rows are selectable. Repeated
/// selections form a union. An empty collection accepts every discovered file;
/// selecting an entire file supersedes restricted lines for that same identity.
#[derive(Clone, Debug, Default)]
pub struct LineSelectors {
    files: std::collections::HashMap<
        std::path::PathBuf,
        Option<std::collections::HashSet<usize>>,
    >,
}

/// Invalid typed line-selector configuration.
#[derive(Debug)]
pub struct LineSelectorError(String);
impl std::fmt::Display for LineSelectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for LineSelectorError {}

impl LineSelectors {
    /// Creates unrestricted source selection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds positive source lines. An empty iterator selects the entire file.
    ///
    /// Existing paths resolve symlinks at configuration time. Missing paths are
    /// valid selectors, resolved lexically against the current working
    /// directory.
    /// They do not add sources to the parser's independent discovery.
    ///
    /// # Errors
    /// Rejects an empty path, zero line, or unavailable working directory.
    pub fn lines(
        mut self,
        path: impl AsRef<std::path::Path>,
        lines: impl IntoIterator<Item = usize>,
    ) -> Result<Self, LineSelectorError> {
        let path = source_identity(path.as_ref())?;
        let lines = lines.into_iter().collect::<std::collections::HashSet<_>>();
        if lines.contains(&0) {
            return Err(LineSelectorError(
                "source lines must be positive (one-based)".into(),
            ));
        }
        let selection = (!lines.is_empty()).then_some(lines);
        self.files
            .entry(path)
            .and_modify(|current| {
                match (current.as_mut(), selection.as_ref()) {
                    (Some(current), Some(lines)) => current.extend(lines),
                    _ => *current = None,
                }
            })
            .or_insert(selection);
        Ok(self)
    }

    /// Selects every scenario from this discovered source.
    ///
    /// # Errors
    /// Rejects an empty path or unavailable working directory.
    pub fn file(
        self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, LineSelectorError> {
        self.lines(path, [])
    }

    pub(crate) fn matches(
        &self,
        path: &std::path::Path,
        origin: &crate::parser::ScenarioOrigin,
    ) -> Result<bool, LineSelectorError> {
        if self.files.is_empty() {
            return Ok(true);
        }
        let path = source_identity(path)?;
        Ok(match self.files.get(&path) {
            None => false,
            Some(None) => true,
            Some(Some(lines)) => {
                lines.contains(&origin.scenario_line())
                    || origin
                        .example_line()
                        .is_some_and(|line| lines.contains(&line))
            }
        })
    }
}

fn source_identity(
    path: &std::path::Path,
) -> Result<std::path::PathBuf, LineSelectorError> {
    use std::path::Component;
    if path.as_os_str().is_empty() {
        return Err(LineSelectorError(
            "source selector path must not be empty".into(),
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| LineSelectorError(error.to_string()))?
            .join(path)
    };
    if let Ok(canonical) = absolute.canonicalize() {
        return Ok(canonical);
    }
    let mut lexical = std::path::PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                lexical.pop();
            }
            part => lexical.push(part.as_os_str()),
        }
    }
    Ok(lexical)
}
