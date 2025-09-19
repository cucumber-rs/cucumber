//! Finished scenarios and features tracking.

use std::collections::HashMap;

use futures::{channel::mpsc, Stream, StreamExt as _};

use crate::{
    Event, World,
    event::{self, source::Source},
};

/// Type alias for the finished features sender.
pub type FinishedFeaturesSender = mpsc::UnboundedSender<(
    Source<gherkin::Feature>,
    Option<Source<gherkin::Rule>>,
    bool,
)>;

/// Type alias for the finished features receiver.
pub type FinishedFeaturesReceiver = mpsc::UnboundedReceiver<(
    Source<gherkin::Feature>,
    Option<Source<gherkin::Rule>>,
    bool,
)>;

/// Tracker for finished rules and features.
///
/// Handles the completion notifications and generates appropriate
/// [`Feature::Finished`] and [`Rule::Finished`] events.
///
/// [`Feature::Finished`]: event::Feature::Finished
/// [`Rule::Finished`]: event::Rule::Finished
pub struct FinishedRulesAndFeatures {
    /// Receiver for finished scenario notifications.
    finished_receiver: FinishedFeaturesReceiver,
    
    /// Tracker for rule scenario counts.
    rule_scenarios_left: HashMap<
        (Source<gherkin::Feature>, Source<gherkin::Rule>),
        usize,
    >,
    
    /// Tracker for feature scenario counts.
    feature_scenarios_left: HashMap<Source<gherkin::Feature>, usize>,
}

impl FinishedRulesAndFeatures {
    /// Creates a new [`FinishedRulesAndFeatures`] tracker.
    pub fn new(finished_receiver: FinishedFeaturesReceiver) -> Self {
        Self {
            finished_receiver,
            rule_scenarios_left: HashMap::new(),
            feature_scenarios_left: HashMap::new(),
        }
    }

    /// Handles a rule scenario finishing and returns a [`Rule::Finished`]
    /// event if all scenarios in the rule are complete.
    ///
    /// [`Rule::Finished`]: event::Rule::Finished
    pub fn rule_scenario_finished<W>(
        &mut self,
        feature: Source<gherkin::Feature>,
        rule: Source<gherkin::Rule>,
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if !is_retried {
            let key = (feature.clone(), rule.clone());
            let scenarios_left = self.rule_scenarios_left.get_mut(&key)?;
            
            *scenarios_left = scenarios_left.saturating_sub(1);
            
            if *scenarios_left == 0 {
                self.rule_scenarios_left.remove(&key);
                return Some(event::Cucumber::feature(
                    feature,
                    event::Feature::rule(rule, event::Rule::Finished),
                ));
            }
        }
        None
    }

    /// Handles a feature scenario finishing and returns a [`Feature::Finished`]
    /// event if all scenarios in the feature are complete.
    ///
    /// [`Feature::Finished`]: event::Feature::Finished
    pub fn feature_scenario_finished<W>(
        &mut self,
        feature: Source<gherkin::Feature>,
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if !is_retried {
            let scenarios_left = self.feature_scenarios_left.get_mut(&feature)?;
            
            *scenarios_left = scenarios_left.saturating_sub(1);
            
            if *scenarios_left == 0 {
                self.feature_scenarios_left.remove(&feature);
                return Some(event::Cucumber::feature(
                    feature,
                    event::Feature::Finished,
                ));
            }
        }
        None
    }

    /// Generates finish events for all remaining rules and features.
    pub fn finish_all_rules_and_features<W>(
        &mut self,
    ) -> impl Iterator<Item = event::Cucumber<W>> {
        let rule_events = self.rule_scenarios_left
            .drain()
            .map(|((feature, rule), _)| {
                event::Cucumber::feature(
                    feature,
                    event::Feature::rule(rule, event::Rule::Finished),
                )
            });

        let feature_events = self.feature_scenarios_left
            .drain()
            .map(|(feature, _)| {
                event::Cucumber::feature(feature, event::Feature::Finished)
            });

        rule_events.chain(feature_events)
    }

    /// Processes scenario start events and returns appropriate events.
    pub fn start_scenarios<W, R>(
        &mut self,
        runnable: R,
    ) -> impl Iterator<Item = event::Cucumber<W>> + use<W, R>
    where
        R: IntoIterator<Item = (
            crate::runner::basic::supporting_structures::ScenarioId,
            Source<gherkin::Feature>,
            Option<Source<gherkin::Rule>>,
            Source<gherkin::Scenario>,
            crate::runner::basic::cli_and_types::ScenarioType,
            Option<crate::event::Retries>,
        )>,
    {
        // Implementation would process the runnable scenarios
        // and generate appropriate start events
        [].into_iter() // Placeholder
    }
}