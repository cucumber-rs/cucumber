//! Modularized scenario storage implementation following Single Responsibility Principle.
//!
//! This module breaks down the large scenario storage implementation into focused components:
//! - `features`: Main Features struct and scenario management
//! - `finished`: Finished scenarios and features tracking
//! - `queue`: Internal queue data structures

mod features;
mod finished;
mod queue;

pub use features::Features;
pub use finished::{
    FinishedRulesAndFeatures,
    FinishedFeaturesSender,
    FinishedFeaturesReceiver,
};
pub use queue::{ScenarioItem, RuleScenarios, ScenarioQueue, FeatureEntry};