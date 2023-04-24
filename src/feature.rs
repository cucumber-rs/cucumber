// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`gherkin::Feature`] extension.

use std::{
    iter, mem,
    path::{Path, PathBuf},
};

use derive_more::{Display, Error};
use lazy_regex::regex;
use once_cell::sync::Lazy;
use regex::Regex;
use sealed::sealed;

use crate::writer::basic::trim_path;

/// Helper methods to operate on [`gherkin::Feature`]s.
#[sealed]
pub trait Ext: Sized {
    /// Expands [`Scenario Outline`][1] [`Examples`][2].
    ///
    /// So this one:
    /// ```gherkin
    /// Feature: Hungry
    ///   Scenario Outline: eating
    ///     Given there are <start> cucumbers
    ///     When I eat <eat> cucumbers
    ///     Then I should have <left> cucumbers
    ///     And substitution in tables works too
    ///      | cucumbers left |
    ///      | <left>         |
    ///
    ///     Examples:
    ///       | start | eat | left |
    ///       |    12 |   5 |    7 |
    ///       |    20 |   4 |   16 |
    /// ```
    ///
    /// Will be expanded as:
    /// ```gherkin
    /// Feature: Hungry
    ///   Scenario Outline: eating
    ///     Given there are 12 cucumbers
    ///     When I eat 5 cucumbers
    ///     Then I should have 7 cucumbers
    ///     And substitution in tables works too
    ///      | cucumbers left |
    ///      | 7              |
    ///   Scenario Outline: eating
    ///     Given there are 20 cucumbers
    ///     When I eat 4 cucumbers
    ///     Then I should have 16 cucumbers
    ///     And substitution in tables works too
    ///      | cucumbers left |
    ///      | 7              |
    ///
    ///     Examples:
    ///       | start | eat | left |
    ///       |    12 |   5 |    7 |
    ///       |    20 |   4 |   16 |
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if the [`Examples`][2] cannot be expanded.
    /// See [`ExpandExamplesError`] for details.
    ///
    /// [1]: https://cucumber.io/docs/gherkin/reference#scenario-outline
    /// [2]: https://cucumber.io/docs/gherkin/reference#examples
    fn expand_examples(self) -> Result<Self, ExpandExamplesError>;

    /// Counts all the [`Feature`]'s [`Scenario`]s, including [`Rule`]s inside.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    fn count_scenarios(&self) -> usize;

    /// Counts all the [`Feature`]'s [`Step`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Step`]: gherkin::Step
    #[must_use]
    fn count_steps(&self) -> usize;
}

#[sealed]
impl Ext for gherkin::Feature {
    fn expand_examples(mut self) -> Result<Self, ExpandExamplesError> {
        let path = self.path.clone();
        let expand = |scenarios: Vec<gherkin::Scenario>| -> Result<_, _> {
            scenarios
                .into_iter()
                .flat_map(|s| expand_scenario(s, path.as_ref()))
                .collect()
        };

        for r in &mut self.rules {
            r.scenarios = expand(mem::take(&mut r.scenarios))?;
        }
        self.scenarios = expand(mem::take(&mut self.scenarios))?;

        Ok(self)
    }

    fn count_scenarios(&self) -> usize {
        self.scenarios.len()
            + self.rules.iter().map(|r| r.scenarios.len()).sum::<usize>()
    }

    fn count_steps(&self) -> usize {
        self.scenarios.iter().map(|s| s.steps.len()).sum::<usize>()
            + self
                .rules
                .iter()
                .flat_map(|r| &r.scenarios)
                .map(|s| s.steps.len())
                .sum::<usize>()
    }
}

/// Expands [`Scenario`] [`Examples`], if any.
///
/// # Errors
///
/// See [`ExpandExamplesError`] for details.
///
/// [`Examples`]: gherkin::Examples
/// [`Scenario`]: gherkin::Scenario
fn expand_scenario(
    scenario: gherkin::Scenario,
    path: Option<&PathBuf>,
) -> Vec<Result<gherkin::Scenario, ExpandExamplesError>> {
    /// [`Regex`] matching placeholders [`Examples`] should expand into.
    ///
    /// [`Examples`]: gherkin::Examples
    static TEMPLATE_REGEX: &Lazy<Regex> = regex!(r"<([^>\s]+)>");

    if scenario.examples.is_empty() {
        return vec![Ok(scenario)];
    }

    scenario
        .examples
        .iter()
        .filter_map(|ex| {
            ex.table
                .as_ref()?
                .rows
                .split_first()
                .map(|(h, v)| (h, v, ex))
        })
        .flat_map(|(header, vals, example)| {
            vals.iter()
                .map(|v| header.iter().zip(v))
                .enumerate()
                .zip(iter::repeat((example.position, example.tags.iter())))
        })
        .map(|((id, row), (position, tags))| {
            let replace_templates = |str: &str, pos| {
                let mut err = None;
                let replaced = TEMPLATE_REGEX
                    .replace_all(str, |cap: &regex::Captures<'_>| {
                        // PANIC: Unwrapping is OK here as `TEMPLATE_REGEX`
                        //        contains this capture group.
                        #[allow(clippy::unwrap_used)]
                        let name = cap.get(1).unwrap().as_str();

                        row.clone()
                            .find_map(|(k, v)| {
                                (name == k).then_some(v.as_str())
                            })
                            .unwrap_or_else(|| {
                                err = Some(ExpandExamplesError {
                                    pos,
                                    name: name.to_owned(),
                                    path: path.cloned(),
                                });
                                ""
                            })
                    })
                    .into_owned();

                err.map_or_else(|| Ok(replaced), Err)
            };

            let mut expanded = scenario.clone();

            // This is done to differentiate `Hash`es of
            // scenario outlines with the same examples.
            expanded.position = position;
            expanded.position.line += id + 2;

            expanded.tags.extend(tags.cloned());

            expanded.name =
                replace_templates(&expanded.name, expanded.position)?;
            for s in &mut expanded.steps {
                for value in iter::once(&mut s.value)
                    .chain(s.docstring.iter_mut())
                    .chain(s.table.iter_mut().flat_map(|t| {
                        t.rows.iter_mut().flat_map(|r| r.iter_mut())
                    }))
                {
                    *value = replace_templates(value, s.position)?;
                }
            }

            Ok(expanded)
        })
        .collect()
}

/// Error of [`Scenario Outline`][1] expansion encountering an unknown template.
///
/// [1]: https://cucumber.io/docs/gherkin/reference#scenario-outline
#[derive(Clone, Debug, Display, Error)]
#[display(
    fmt = "Failed to resolve <{}> at {}:{}:{}",
    name,
    "path.as_deref().and_then(Path::to_str).map(trim_path).unwrap_or_default()",
    "pos.line",
    "pos.col"
)]
pub struct ExpandExamplesError {
    /// Position of the unknown template.
    pub pos: gherkin::LineCol,

    /// Name of the unknown template.
    pub name: String,

    /// [`Path`] to the `.feature` file, if present.
    pub path: Option<PathBuf>,
}
