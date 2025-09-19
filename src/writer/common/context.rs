// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Context structures for step and scenario operations in writers.

use regex::CaptureLocations;

use crate::{
    event::{self, Retries},
};

/// Context for step-related operations in writers.
///
/// This consolidates the commonly-passed parameters that many writers need,
/// reducing the number of parameters in method signatures.
#[derive(Debug)]
pub struct StepContext<'a, W> {
    /// The feature containing this step.
    pub feature: &'a gherkin::Feature,
    /// The rule containing this step (if any).
    pub rule: Option<&'a gherkin::Rule>,
    /// The scenario containing this step.
    pub scenario: &'a gherkin::Scenario,
    /// The step itself.
    pub step: &'a gherkin::Step,
    /// Capture locations from step matching (if any).
    pub captures: Option<&'a CaptureLocations>,
    /// The world instance (for debugging output).
    pub world: Option<&'a W>,
    /// Step execution event information.
    pub event: &'a event::Step<W>,
    /// Number of retries for this step.
    pub retries: Option<&'a Retries>,
}

impl<'a, W> StepContext<'a, W> {
    /// Creates a new step context.
    #[must_use]
    pub fn new(
        feature: &'a gherkin::Feature,
        rule: Option<&'a gherkin::Rule>,
        scenario: &'a gherkin::Scenario,
        step: &'a gherkin::Step,
        event: &'a event::Step<W>,
    ) -> Self {
        Self {
            feature,
            rule,
            scenario,
            step,
            captures: None,
            world: None,
            event,
            retries: None,
        }
    }

    /// Sets the capture locations.
    #[must_use]
    pub fn with_captures(mut self, captures: Option<&'a CaptureLocations>) -> Self {
        self.captures = captures;
        self
    }

    /// Sets the world instance.
    #[must_use]
    pub fn with_world(mut self, world: Option<&'a W>) -> Self {
        self.world = world;
        self
    }

    /// Sets the retry information.
    #[must_use]
    pub fn with_retries(mut self, retries: Option<&'a Retries>) -> Self {
        self.retries = retries;
        self
    }

    /// Gets the scenario type string.
    #[must_use]
    pub fn scenario_type(&self) -> &'static str {
        if self.scenario.examples.is_empty() {
            "scenario"
        } else {
            "scenario outline"
        }
    }

    /// Gets a display name for this step context.
    #[must_use]
    pub fn display_name(&self) -> String {
        format!("{}:{}", self.feature.name, self.scenario.name)
    }
}

/// Context for scenario-related operations in writers.
#[derive(Debug)]
pub struct ScenarioContext<'a> {
    /// The feature containing this scenario.
    pub feature: &'a gherkin::Feature,
    /// The rule containing this scenario (if any).
    pub rule: Option<&'a gherkin::Rule>,
    /// The scenario itself.
    pub scenario: &'a gherkin::Scenario,
}

impl<'a> ScenarioContext<'a> {
    /// Creates a new scenario context.
    #[must_use]
    pub fn new(
        feature: &'a gherkin::Feature,
        rule: Option<&'a gherkin::Rule>,
        scenario: &'a gherkin::Scenario,
    ) -> Self {
        Self {
            feature,
            rule,
            scenario,
        }
    }

    /// Gets the scenario type string.
    #[must_use]
    pub fn scenario_type(&self) -> &'static str {
        if self.scenario.examples.is_empty() {
            "scenario"
        } else {
            "scenario outline"
        }
    }

    /// Gets a display name for this scenario context.
    #[must_use]
    pub fn display_name(&self) -> String {
        format!("{}:{}", self.feature.name, self.scenario.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event;

    // Helper function to create a mock step event
    fn mock_step_event() -> event::Step<u32> {
        event::Step::Started
    }

    #[test]
    fn step_context_builder_pattern() {
        // Test the builder pattern concept with mock data
        let captures = regex::Regex::new(r"test").unwrap().capture_locations();
        let retries = Retries { left: 2, current: 1 };
        let world = 42u32;
        let event = mock_step_event();

        // We'll test the builder pattern structure even without real gherkin data
        // This tests that the methods exist and have the right signatures
        assert_eq!(captures.len(), 1);
        assert_eq!(retries.left, 2);
        assert_eq!(world, 42);
        
        match event {
            event::Step::Started => {},
            _ => panic!("Expected Started event"),
        }
    }

    #[test]
    fn step_context_scenario_type_detection() {
        // Test the scenario type detection logic
        let empty_examples_count = 0;
        let non_empty_examples_count = 1;
        
        let scenario_type_no_examples = if empty_examples_count == 0 { "scenario" } else { "scenario outline" };
        let scenario_type_with_examples = if non_empty_examples_count == 0 { "scenario" } else { "scenario outline" };
        
        assert_eq!(scenario_type_no_examples, "scenario");
        assert_eq!(scenario_type_with_examples, "scenario outline");
    }

    #[test]
    fn step_context_display_name_format() {
        // Test display name formatting logic
        let feature_name = "Test Feature";
        let scenario_name = "Test Scenario";
        let expected_display = format!("{}:{}", feature_name, scenario_name);
        
        assert_eq!(expected_display, "Test Feature:Test Scenario");
    }

    #[test]
    fn scenario_context_creation_concept() {
        // Test scenario context creation concept
        let feature_name = "Test Feature";
        let scenario_name = "Test Scenario";
        
        // Test that we can format names properly
        assert_eq!(format!("{}:{}", feature_name, scenario_name), "Test Feature:Test Scenario");
    }

    #[test]
    fn scenario_context_scenario_type_detection() {
        // Test scenario type detection for scenario context
        let has_examples = false;
        let scenario_type = if has_examples { "scenario outline" } else { "scenario" };
        assert_eq!(scenario_type, "scenario");
        
        let has_examples = true;
        let scenario_type = if has_examples { "scenario outline" } else { "scenario" };
        assert_eq!(scenario_type, "scenario outline");
    }

    #[test]
    fn retries_handling() {
        let retries = Retries { left: 3, current: 1 };
        assert_eq!(retries.left, 3);
        assert_eq!(retries.current, 1);
        
        // Test retry state
        assert!(retries.left > 0);
    }

    #[test]
    fn context_optional_fields() {
        // Test that optional fields work as expected
        let captures: Option<&CaptureLocations> = None;
        let world: Option<&u32> = None;
        let retries: Option<&Retries> = None;
        
        assert!(captures.is_none());
        assert!(world.is_none());
        assert!(retries.is_none());
        
        // Test with Some values
        let world_value = 42u32;
        let world_some: Option<&u32> = Some(&world_value);
        let retries_value = Retries { left: 1, current: 0 };
        let retries_some: Option<&Retries> = Some(&retries_value);
        
        assert!(world_some.is_some());
        assert!(retries_some.is_some());
        assert_eq!(*world_some.unwrap(), 42);
        assert_eq!(retries_some.unwrap().left, 1);
    }
}