// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Feature structure and utilities for Cucumber JSON format.

use serde::Serialize;

use crate::{
    feature::ExpandExamplesError,
    writer::{
        basic::trim_path,
        json::{element::Element, types::{RunResult, Status, Step, Tag}},
    },
};

/// [`Serialize`]able [`gherkin::Feature`].
#[derive(Clone, Debug, Serialize)]
pub struct Feature {
    /// [`gherkin::Feature::path`].
    pub uri: Option<String>,

    /// [`gherkin::Feature::keyword`].
    pub keyword: String,

    /// [`gherkin::Feature::name`].
    pub name: String,

    /// [`gherkin::Feature::tags`].
    pub tags: Vec<Tag>,

    /// [`gherkin::Feature`]'s [`Element`]s.
    pub elements: Vec<Element>,
}

impl Feature {
    /// Creates a new [`Feature`] out of the given [`gherkin::Feature`].
    pub fn new(feature: &gherkin::Feature) -> Self {
        Self {
            uri: feature
                .path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .map(str::to_owned),
            keyword: feature.keyword.clone(),
            name: feature.name.clone(),
            tags: feature
                .tags
                .iter()
                .map(|tag| Tag {
                    name: tag.clone(),
                    line: feature.position.line,
                })
                .collect(),
            elements: vec![],
        }
    }

    /// Creates a new [`Feature`] from the given [`ExpandExamplesError`].
    pub fn example_expansion_err(err: &ExpandExamplesError) -> Self {
        Self {
            uri: err
                .path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .map(str::to_owned),
            keyword: String::new(),
            name: String::new(),
            tags: vec![],
            elements: vec![Element {
                after: vec![],
                before: vec![],
                keyword: String::new(),
                r#type: "scenario",
                id: format!(
                    "failed-to-expand-examples{}",
                    err.path
                        .as_ref()
                        .and_then(|p| p.to_str().map(trim_path))
                        .unwrap_or_default(),
                ),
                line: 0,
                name: String::new(),
                tags: vec![],
                steps: vec![Step {
                    keyword: String::new(),
                    line: err.pos.line,
                    name: "scenario".into(),
                    hidden: false,
                    result: RunResult {
                        status: Status::Failed,
                        duration: 0,
                        error_message: Some(err.to_string()),
                    },
                    embeddings: vec![],
                }],
            }],
        }
    }

    /// Creates a new [`Feature`] from the given [`gherkin::ParseFileError`].
    pub fn parsing_err(err: &gherkin::ParseFileError) -> Self {
        let path = match err {
            gherkin::ParseFileError::Reading { path, .. }
            | gherkin::ParseFileError::Parsing { path, .. } => path,
        }
        .to_str()
        .map(trim_path)
        .map(str::to_owned);

        Self {
            uri: path.clone(),
            keyword: String::new(),
            name: String::new(),
            tags: vec![],
            elements: vec![Element {
                after: vec![],
                before: vec![],
                keyword: String::new(),
                r#type: "scenario",
                id: format!(
                    "failed-to-parse{}",
                    path.as_deref().unwrap_or_default(),
                ),
                line: 0,
                name: String::new(),
                tags: vec![],
                steps: vec![Step {
                    keyword: String::new(),
                    line: 0,
                    name: "scenario".into(),
                    hidden: false,
                    result: RunResult {
                        status: Status::Failed,
                        duration: 0,
                        error_message: Some(err.to_string()),
                    },
                    embeddings: vec![],
                }],
            }],
        }
    }

    /// Finds an element matching the given parameters.
    pub fn find_element(
        &self,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> Option<&Element> {
        self.elements
            .iter()
            .find(|el| el.matches_scenario(rule, scenario, ty))
    }

    /// Finds a mutable element matching the given parameters.
    pub fn find_element_mut(
        &mut self,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> Option<&mut Element> {
        self.elements
            .iter_mut()
            .find(|el| el.matches_scenario(rule, scenario, ty))
    }

    /// Returns the total number of elements in this feature.
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Returns whether this feature has any elements.
    pub fn has_elements(&self) -> bool {
        !self.elements.is_empty()
    }
}

impl PartialEq<gherkin::Feature> for Feature {
    fn eq(&self, other: &gherkin::Feature) -> bool {
        self.uri
            .as_ref()
            .and_then(|uri| {
                other
                    .path
                    .as_ref()
                    .and_then(|p| p.to_str().map(trim_path))
                    .map(|path| uri == path)
            })
            .unwrap_or_default()
            && self.name == other.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gherkin::{Feature as GherkinFeature, LineCol, Rule, Scenario};
    use std::path::PathBuf;

    fn create_test_gherkin_feature() -> GherkinFeature {
        GherkinFeature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec!["@feature-tag".to_string()],
            position: LineCol { line: 1, col: 1 },
            path: Some(PathBuf::from("features/test.feature")),
        }
    }

    fn create_test_scenario() -> Scenario {
        Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            tags: vec!["@scenario-tag".to_string()],
            position: LineCol { line: 5, col: 1 },
            steps: vec![],
            examples: vec![],
        }
    }

    #[test]
    fn feature_new_from_gherkin() {
        let gherkin_feature = create_test_gherkin_feature();
        let feature = Feature::new(&gherkin_feature);
        
        assert_eq!(feature.uri, Some("features/test.feature".to_string()));
        assert_eq!(feature.keyword, "Feature");
        assert_eq!(feature.name, "Test Feature");
        assert_eq!(feature.tags.len(), 1);
        assert_eq!(feature.tags[0].name, "@feature-tag");
        assert_eq!(feature.tags[0].line, 1);
        assert!(feature.elements.is_empty());
    }

    #[test]
    fn feature_example_expansion_error() {
        let error = ExpandExamplesError {
            path: Some(PathBuf::from("features/error.feature")),
            pos: LineCol { line: 10, col: 5 },
        };
        
        let feature = Feature::example_expansion_err(&error);
        
        assert_eq!(feature.uri, Some("features/error.feature".to_string()));
        assert_eq!(feature.keyword, "");
        assert_eq!(feature.name, "");
        assert!(feature.tags.is_empty());
        assert_eq!(feature.elements.len(), 1);
        
        let element = &feature.elements[0];
        assert_eq!(element.r#type, "scenario");
        assert_eq!(element.id, "failed-to-expand-examplesfeatures/error.feature");
        assert_eq!(element.steps.len(), 1);
        
        let step = &element.steps[0];
        assert_eq!(step.line, 10);
        assert_eq!(step.name, "scenario");
        assert_eq!(step.result.status, Status::Failed);
        assert!(step.result.error_message.is_some());
    }

    #[test]
    fn feature_parsing_error() {
        let error = gherkin::ParseFileError::Reading {
            path: PathBuf::from("features/bad.feature"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        };
        
        let feature = Feature::parsing_err(&error);
        
        assert_eq!(feature.uri, Some("features/bad.feature".to_string()));
        assert_eq!(feature.keyword, "");
        assert_eq!(feature.name, "");
        assert!(feature.tags.is_empty());
        assert_eq!(feature.elements.len(), 1);
        
        let element = &feature.elements[0];
        assert_eq!(element.r#type, "scenario");
        assert_eq!(element.id, "failed-to-parsefeatures/bad.feature");
        assert_eq!(element.steps.len(), 1);
        
        let step = &element.steps[0];
        assert_eq!(step.line, 0);
        assert_eq!(step.name, "scenario");
        assert_eq!(step.result.status, Status::Failed);
        assert!(step.result.error_message.is_some());
    }

    #[test]
    fn feature_find_element() {
        let gherkin_feature = create_test_gherkin_feature();
        let scenario = create_test_scenario();
        let mut feature = Feature::new(&gherkin_feature);
        
        // Add an element
        feature.elements.push(Element::new(&gherkin_feature, None, &scenario, "scenario"));
        
        // Test finding the element
        assert!(feature.find_element(None, &scenario, "scenario").is_some());
        assert!(feature.find_element(None, &scenario, "background").is_none());
    }

    #[test]
    fn feature_find_element_mut() {
        let gherkin_feature = create_test_gherkin_feature();
        let scenario = create_test_scenario();
        let mut feature = Feature::new(&gherkin_feature);
        
        // Add an element
        feature.elements.push(Element::new(&gherkin_feature, None, &scenario, "scenario"));
        
        // Test finding mutable element
        let element = feature.find_element_mut(None, &scenario, "scenario");
        assert!(element.is_some());
        
        // Modify the element to test mutability
        if let Some(element) = element {
            element.name = "Modified Name".to_string();
        }
        
        assert_eq!(feature.elements[0].name, "Modified Name");
    }

    #[test]
    fn feature_element_count_and_has_elements() {
        let gherkin_feature = create_test_gherkin_feature();
        let scenario = create_test_scenario();
        let mut feature = Feature::new(&gherkin_feature);
        
        assert_eq!(feature.element_count(), 0);
        assert!(!feature.has_elements());
        
        feature.elements.push(Element::new(&gherkin_feature, None, &scenario, "scenario"));
        
        assert_eq!(feature.element_count(), 1);
        assert!(feature.has_elements());
    }

    #[test]
    fn feature_partial_eq_with_gherkin() {
        let gherkin_feature = create_test_gherkin_feature();
        let feature = Feature::new(&gherkin_feature);
        
        assert_eq!(feature, gherkin_feature);
        
        // Test with different name
        let mut different_feature = gherkin_feature.clone();
        different_feature.name = "Different Name".to_string();
        assert_ne!(feature, different_feature);
    }

    #[test]
    fn feature_serialization() {
        let gherkin_feature = create_test_gherkin_feature();
        let feature = Feature::new(&gherkin_feature);
        
        let json = serde_json::to_value(&feature).unwrap();
        
        assert_eq!(json["uri"], "features/test.feature");
        assert_eq!(json["keyword"], "Feature");
        assert_eq!(json["name"], "Test Feature");
        assert_eq!(json["tags"].as_array().unwrap().len(), 1);
        assert_eq!(json["elements"].as_array().unwrap().len(), 0);
    }
}