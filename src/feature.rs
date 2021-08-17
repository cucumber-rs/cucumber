// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`gherkin::Feature`] extension.

use std::{iter, mem};

use sealed::sealed;

/// Helper methods to operate on [`gherkin::Feature`]s.
#[sealed]
pub trait Ext: Sized {
    /// Expands [Scenario outline][1] [examples][2].
    ///
    /// So this one:
    /// ```gherkin
    /// Feature: Hungry
    ///   Scenario Outline: eating
    ///     Given there are <start> cucumbers
    ///     When I eat <eat> cucumbers
    ///     Then I should have <left> cucumbers
    ///
    ///     Examples:
    ///       | start | eat | left |
    ///       |    12 |   5 |    7 |
    ///       |    20 |   5 |   15 |
    /// ```
    ///
    /// Will be expanded as:
    /// ```gherkin
    /// Feature: Hungry
    ///   Scenario Outline: eating
    ///     Given there are 12 cucumbers
    ///     When I eat 5 cucumbers
    ///     Then I should have 7 cucumbers
    ///   Scenario Outline: eating
    ///     Given there are 20 cucumbers
    ///     When I eat 5 cucumbers
    ///     Then I should have 15 cucumbers
    ///
    ///     Examples:
    ///       | start | eat | left |
    ///       |    12 |   5 |    7 |
    ///       |    20 |   5 |   15 |
    /// ```
    ///
    /// [1]: https://cucumber.io/docs/gherkin/reference/#scenario-outline
    /// [2]: https://cucumber.io/docs/gherkin/reference/#examples
    fn expand_examples(self) -> Self;

    /// Counts all the [`Feature`]'s [`Scenario`]s, including [`Rule`]s inside.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    fn count_scenarios(&self) -> usize;
}

#[sealed]
impl Ext for gherkin::Feature {
    fn expand_examples(mut self) -> Self {
        let scenarios = mem::take(&mut self.scenarios)
            .into_iter()
            .flat_map(|scenario| {
                let ((header, vals), examples) =
                    match scenario.examples.as_ref().and_then(|ex| {
                        ex.table.rows.split_first().map(|t| (t, ex))
                    }) {
                        Some(s) => s,
                        None => return vec![scenario],
                    };

                vals.iter()
                    .zip(iter::repeat_with(|| header))
                    .enumerate()
                    .map(|(id, (vals, keys))| {
                        let mut modified = scenario.clone();

                        // This is done to differentiate `Hash`es of Scenario
                        // outlines with the same examples.
                        modified.position = examples.position;
                        modified.position.line += id + 1;

                        for step in &mut modified.steps {
                            for (key, val) in keys.iter().zip(vals) {
                                step.value = step
                                    .value
                                    .replace(&format!("<{}>", key), val);
                            }
                        }
                        modified
                    })
                    .collect()
            })
            .collect();

        self.scenarios = scenarios;
        self
    }

    fn count_scenarios(&self) -> usize {
        self.scenarios.len()
            + self.rules.iter().map(|r| r.scenarios.len()).sum::<usize>()
    }
}
