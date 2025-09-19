//! Internal queue structures for scenario storage.

use std::collections::{HashMap, VecDeque};

use crate::{
    event::{source::Source, Retries},
    runner::basic::{
        cli_and_types::ScenarioType,
        supporting_structures::ScenarioId,
    },
};

/// Queue item representing a scenario with its metadata.
pub type ScenarioItem = (
    ScenarioId,
    Source<gherkin::Feature>,
    Option<Source<gherkin::Rule>>,
    Source<gherkin::Scenario>,
    ScenarioType,
    Option<Retries>,
);

/// Queue for managing scenarios within a rule.
pub type RuleScenarios = VecDeque<ScenarioItem>;

/// Main scenario queue organizing features and their scenarios.
pub type ScenarioQueue = HashMap<Source<gherkin::Feature>, FeatureEntry>;

/// Entry for a feature containing its rules and scenarios.
pub struct FeatureEntry {
    /// Direct scenarios in the feature (not under any rule).
    pub scenarios: VecDeque<ScenarioItem>,
    
    /// Rules and their scenarios.
    pub rules: HashMap<Source<gherkin::Rule>, RuleScenarios>,
}

impl Default for FeatureEntry {
    fn default() -> Self {
        Self {
            scenarios: VecDeque::new(),
            rules: HashMap::new(),
        }
    }
}

impl FeatureEntry {
    /// Creates a new empty feature entry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a scenario to this feature entry.
    pub fn add_scenario(&mut self, scenario: ScenarioItem) {
        self.scenarios.push_back(scenario);
    }

    /// Adds a scenario under a specific rule.
    pub fn add_rule_scenario(
        &mut self,
        rule: Source<gherkin::Rule>,
        scenario: ScenarioItem,
    ) {
        self.rules
            .entry(rule)
            .or_insert_with(VecDeque::new)
            .push_back(scenario);
    }

    /// Gets the next scenario to execute, prioritizing serial scenarios.
    pub fn next_scenario(&mut self) -> Option<ScenarioItem> {
        // First check direct scenarios
        if let Some(scenario) = self.scenarios.pop_front() {
            return Some(scenario);
        }

        // Then check rule scenarios
        for rule_scenarios in self.rules.values_mut() {
            if let Some(scenario) = rule_scenarios.pop_front() {
                return Some(scenario);
            }
        }

        None
    }

    /// Checks if this feature entry has any remaining scenarios.
    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty() 
            && self.rules.values().all(|scenarios| scenarios.is_empty())
    }

    /// Counts total scenarios in this feature.
    pub fn total_scenarios(&self) -> usize {
        self.scenarios.len() 
            + self.rules.values().map(|scenarios| scenarios.len()).sum::<usize>()
    }
}