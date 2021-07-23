//! [`Feature`] extension trait.
//!
//! [`Feature`]: gherkin::Feature

use std::iter;

use sealed::sealed;

/// Some helper-methods to operate with [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
#[sealed]
pub trait FeatureExt: Sized {
    /// Expands [Scenario Outline][1] [Examples][2].
    ///
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
    /// Will be expanded to:
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

    /// Counts all [`Feature`]'s [`Scenario`]s, including inside [`Rule`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    fn count_scenarios(&self) -> usize;
}

#[sealed]
impl FeatureExt for gherkin::Feature {
    fn expand_examples(mut self) -> Self {
        let scenarios = std::mem::take(&mut self.scenarios);
        let scenarios = scenarios
            .into_iter()
            .flat_map(|scenario| {
                let ((header, values), examples) =
                    match scenario.examples.as_ref().and_then(|ex| {
                        ex.table.rows.split_first().map(|t| (t, ex))
                    }) {
                        Some(s) => s,
                        None => return vec![scenario],
                    };

                values
                    .iter()
                    .zip(iter::repeat_with(|| header))
                    .enumerate()
                    .map(|(id, (values, keys))| {
                        let mut modified = scenario.clone();

                        // This is done to differentiate `Hash`es of Scenario
                        // Outlines with the same examples.
                        modified.position = examples.position;
                        modified.position.line += id + 1;

                        for step in &mut modified.steps {
                            for (key, value) in keys.iter().zip(values) {
                                step.value = step
                                    .value
                                    .replace(&format!("<{}>", key), value);
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
