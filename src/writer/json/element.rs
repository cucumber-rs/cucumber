// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Element (Scenario/Background) structures and utilities for Cucumber JSON format.

use inflector::Inflector as _;
use serde::Serialize;

use crate::writer::json::types::{HookResult, Step, Tag};

/// [`Serialize`]able [`gherkin::Background`] or [`gherkin::Scenario`].
#[derive(Clone, Debug, Serialize)]
pub struct Element {
    /// Doesn't appear in the [JSON schema][1], but present in
    /// [its generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<HookResult>,

    /// Doesn't appear in the [JSON schema][1], but present in
    /// [its generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<HookResult>,

    /// [`gherkin::Scenario::keyword`].
    pub keyword: String,

    /// Type of this [`Element`].
    ///
    /// Only set to `background` or `scenario`, but [JSON schema][1] doesn't
    /// constraint only to those values, so maybe a subject to change.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    pub r#type: &'static str,

    /// Identifier of this [`Element`]. Doesn't have to be unique.
    pub id: String,

    /// [`gherkin::Scenario`] line number inside a `.feature` file.
    pub line: usize,

    /// [`gherkin::Scenario::name`], optionally prepended with a
    /// [`gherkin::Rule::name`].
    ///
    /// This is done because [JSON schema][1] doesn't support [`gherkin::Rule`]s
    /// at the moment.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    pub name: String,

    /// [`gherkin::Scenario::tags`].
    pub tags: Vec<Tag>,

    /// [`gherkin::Scenario`]'s [`Step`]s.
    pub steps: Vec<Step>,
}

impl Element {
    /// Creates a new [`Element`] out of the given values.
    pub fn new(
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> Self {
        Self {
            after: vec![],
            before: vec![],
            keyword: (ty == "background")
                .then(|| feature.background.as_ref().map(|bg| &bg.keyword))
                .flatten()
                .unwrap_or(&scenario.keyword)
                .clone(),
            r#type: ty,
            id: format!(
                "{}{}/{}",
                feature.name.to_kebab_case(),
                rule.map(|r| format!("/{}", r.name.to_kebab_case()))
                    .unwrap_or_default(),
                scenario.name.to_kebab_case(),
            ),
            line: scenario.position.line,
            name: format!(
                "{}{}",
                rule.map(|r| format!("{} ", r.name)).unwrap_or_default(),
                scenario.name.clone(),
            ),
            tags: scenario
                .tags
                .iter()
                .map(|t| Tag { name: t.clone(), line: scenario.position.line })
                .collect(),
            steps: vec![],
        }
    }

    /// Checks if this element matches the given scenario parameters.
    pub fn matches_scenario(
        &self,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> bool {
        self.name
            == format!(
                "{}{}",
                rule.map(|r| format!("{} ", r.name))
                    .unwrap_or_default(),
                scenario.name,
            )
            && self.line == scenario.position.line
            && self.r#type == ty
    }

    /// Returns whether this element has any before hooks.
    pub fn has_before_hooks(&self) -> bool {
        !self.before.is_empty()
    }

    /// Returns whether this element has any after hooks.
    pub fn has_after_hooks(&self) -> bool {
        !self.after.is_empty()
    }

    /// Returns the total number of steps in this element.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gherkin::{Feature, Rule, Scenario};

    fn create_test_feature() -> Feature {
        Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        }
    }

    fn create_test_scenario() -> Scenario {
        Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            tags: vec!["@tag1".to_string(), "@tag2".to_string()],
            position: gherkin::LineCol { line: 5, col: 1 },
            steps: vec![],
            examples: vec![],
        }
    }

    fn create_test_rule() -> Rule {
        Rule {
            keyword: "Rule".to_string(),
            name: "Test Rule".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            position: gherkin::LineCol { line: 3, col: 1 },
        }
    }

    #[test]
    fn element_new_scenario_without_rule() {
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        
        let element = Element::new(&feature, None, &scenario, "scenario");
        
        assert_eq!(element.keyword, "Scenario");
        assert_eq!(element.r#type, "scenario");
        assert_eq!(element.id, "test-feature/test-scenario");
        assert_eq!(element.line, 5);
        assert_eq!(element.name, "Test Scenario");
        assert_eq!(element.tags.len(), 2);
        assert_eq!(element.tags[0].name, "@tag1");
        assert_eq!(element.tags[0].line, 5);
        assert!(element.before.is_empty());
        assert!(element.after.is_empty());
        assert!(element.steps.is_empty());
    }

    #[test]
    fn element_new_scenario_with_rule() {
        let feature = create_test_feature();
        let rule = create_test_rule();
        let scenario = create_test_scenario();
        
        let element = Element::new(&feature, Some(&rule), &scenario, "scenario");
        
        assert_eq!(element.id, "test-feature/test-rule/test-scenario");
        assert_eq!(element.name, "Test Rule Test Scenario");
    }

    #[test]
    fn element_new_background() {
        let mut feature = create_test_feature();
        feature.background = Some(gherkin::Background {
            keyword: "Background".to_string(),
            description: None,
            steps: vec![],
            position: gherkin::LineCol { line: 2, col: 1 },
        });
        let scenario = create_test_scenario();
        
        let element = Element::new(&feature, None, &scenario, "background");
        
        assert_eq!(element.keyword, "Background");
        assert_eq!(element.r#type, "background");
    }

    #[test]
    fn element_matches_scenario_without_rule() {
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let element = Element::new(&feature, None, &scenario, "scenario");
        
        assert!(element.matches_scenario(None, &scenario, "scenario"));
        assert!(!element.matches_scenario(None, &scenario, "background"));
    }

    #[test]
    fn element_matches_scenario_with_rule() {
        let feature = create_test_feature();
        let rule = create_test_rule();
        let scenario = create_test_scenario();
        let element = Element::new(&feature, Some(&rule), &scenario, "scenario");
        
        assert!(element.matches_scenario(Some(&rule), &scenario, "scenario"));
        assert!(!element.matches_scenario(None, &scenario, "scenario"));
    }

    #[test]
    fn element_hook_checks() {
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let mut element = Element::new(&feature, None, &scenario, "scenario");
        
        assert!(!element.has_before_hooks());
        assert!(!element.has_after_hooks());
        
        element.before.push(HookResult {
            result: crate::writer::json::types::RunResult {
                status: crate::writer::json::types::Status::Passed,
                duration: 1000,
                error_message: None,
            },
            embeddings: vec![],
        });
        
        assert!(element.has_before_hooks());
        assert!(!element.has_after_hooks());
    }

    #[test]
    fn element_step_count() {
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let mut element = Element::new(&feature, None, &scenario, "scenario");
        
        assert_eq!(element.step_count(), 0);
        
        element.steps.push(crate::writer::json::types::Step {
            keyword: "Given".to_string(),
            line: 6,
            name: "a test step".to_string(),
            hidden: false,
            result: crate::writer::json::types::RunResult {
                status: crate::writer::json::types::Status::Passed,
                duration: 1000,
                error_message: None,
            },
            embeddings: vec![],
        });
        
        assert_eq!(element.step_count(), 1);
    }

    #[test]
    fn element_serialization() {
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let element = Element::new(&feature, None, &scenario, "scenario");
        
        let json = serde_json::to_value(&element).unwrap();
        
        assert_eq!(json["keyword"], "Scenario");
        assert_eq!(json["type"], "scenario");
        assert_eq!(json["id"], "test-feature/test-scenario");
        assert_eq!(json["line"], 5);
        assert_eq!(json["name"], "Test Scenario");
        
        // Empty vectors should be omitted
        assert!(!json.as_object().unwrap().contains_key("before"));
        assert!(!json.as_object().unwrap().contains_key("after"));
    }
}