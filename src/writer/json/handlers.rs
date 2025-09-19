// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Event handling utilities for JSON writer.

use std::{mem, time::SystemTime};

use crate::{
    event::{self, HookType, Metadata, Scenario},
    writer::{
        basic::coerce_error,
        common::{StepContext, WriterStats},
        json::{
            element::Element,
            feature::Feature,
            types::{Embedding, HookResult, RunResult, Status, Step},
        },
    },
};

/// Handler for processing Cucumber events and updating JSON structures.
#[derive(Clone, Debug)]
pub struct EventHandler {
    /// Collection of [`Feature`]s to output [JSON][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    pub features: Vec<Feature>,

    /// [`SystemTime`] when the current [`Hook`]/[`Step`] has started.
    ///
    /// [`Hook`]: event::Hook
    pub started: Option<SystemTime>,

    /// [`event::Scenario::Log`]s of the current [`Hook`]/[`Step`].
    ///
    /// [`Hook`]: event::Hook
    pub logs: Vec<String>,

    /// Statistics tracking using consolidated utilities.
    pub stats: WriterStats,
}

impl EventHandler {
    /// Creates a new [`EventHandler`].
    pub fn new() -> Self {
        Self {
            features: vec![],
            started: None,
            logs: vec![],
            stats: WriterStats::new(),
        }
    }

    /// Handles the given [`event::Scenario`].
    pub fn handle_scenario_event<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: event::Scenario<W>,
        meta: event::Metadata,
    ) {
        match ev {
            Scenario::Started => {}
            Scenario::Hook(ty, ev) => {
                self.handle_hook_event(feature, rule, scenario, ty, ev, meta);
            }
            Scenario::Background(st, ev) => {
                let context = StepContext::new(feature, rule, scenario, &st, &ev);
                self.handle_step_event_with_context(&context, "background", meta);
            }
            Scenario::Step(st, ev) => {
                let context = StepContext::new(feature, rule, scenario, &st, &ev);
                self.handle_step_event_with_context(&context, "scenario", meta);
            }
            Scenario::Log(msg) => {
                self.logs.push(msg);
            }
            Scenario::Finished => {
                self.logs.clear();
            }
        }
    }

    /// Handles the given [`event::Hook`].
    pub fn handle_hook_event<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        hook_ty: event::HookType,
        event: event::Hook<W>,
        meta: event::Metadata,
    ) {
        use event::{Hook, HookType};

        let mut duration = || {
            let started = match self.started.take() {
                Some(started) => started,
                None => {
                    eprintln!("Warning: no `Started` event for `{hook_ty} Hook`");
                    return 0;
                }
            };
            meta.at
                .duration_since(started)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Warning: Failed to compute duration between {:?} and \
                         {started:?}: {e}",
                        meta.at,
                    );
                    std::time::Duration::ZERO
                })
                .as_nanos()
        };

        let res = match event {
            Hook::Started => {
                self.started = Some(meta.at);
                return;
            }
            Hook::Passed => HookResult {
                result: RunResult {
                    status: Status::Passed,
                    duration: duration(),
                    error_message: None,
                },
                embeddings: mem::take(&mut self.logs)
                    .into_iter()
                    .map(Embedding::from_log)
                    .collect(),
            },
            Hook::Failed(_, info) => HookResult {
                result: RunResult {
                    status: Status::Failed,
                    duration: duration(),
                    error_message: Some(coerce_error(&info).into_owned()),
                },
                embeddings: mem::take(&mut self.logs)
                    .into_iter()
                    .map(Embedding::from_log)
                    .collect(),
            },
        };

        let el = self.mut_or_insert_element(feature, rule, scenario, "scenario");
        match hook_ty {
            HookType::Before => el.before.push(res),
            HookType::After => el.after.push(res),
        }
    }

    /// Handles the given [`event::Step`] with consolidated context.
    pub fn handle_step_event_with_context<W>(
        &mut self,
        context: &StepContext<'_, W>,
        ty: &'static str,
        meta: event::Metadata,
    ) {
        let feature = context.feature;
        let rule = context.rule;
        let scenario = context.scenario;
        let step = context.step;
        let event = context.event;
        let mut duration = || {
            let started = match self.started.take() {
                Some(started) => started,
                None => {
                    eprintln!("Warning: no `Started` event for `Step` '{}'", step.value);
                    return 0;
                }
            };
            meta.at
                .duration_since(started)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "Warning: failed to compute duration between {:?} and \
                         {started:?}: {e}",
                        meta.at,
                    );
                    std::time::Duration::ZERO
                })
                .as_nanos()
        };

        let result = match event {
            event::Step::Started => {
                self.started = Some(meta.at);
                _ = self.mut_or_insert_element(feature, rule, scenario, ty);
                return;
            }
            event::Step::Passed(..) => {
                self.stats.record_passed_step();
                RunResult {
                    status: Status::Passed,
                    duration: duration(),
                    error_message: None,
                }
            }
            event::Step::Failed(_, loc, _, err) => {
                self.stats.record_failed_step();
                let status = match &err {
                    event::StepError::NotFound => Status::Undefined,
                    event::StepError::AmbiguousMatch(..) => Status::Ambiguous,
                    event::StepError::Panic(..) => Status::Failed,
                };
                RunResult {
                    status,
                    duration: duration(),
                    error_message: Some(format!(
                        "{}{err}",
                        loc.map(|l| format!(
                            "Matched: {}:{}:{}\n",
                            l.path, l.line, l.column,
                        ))
                        .unwrap_or_default(),
                    )),
                }
            }
            event::Step::Skipped => {
                self.stats.record_skipped_step();
                RunResult {
                    status: Status::Skipped,
                    duration: duration(),
                    error_message: None,
                }
            }
        };

        let step = Step {
            keyword: step.keyword.clone(),
            line: step.position.line,
            name: step.value.clone(),
            hidden: false,
            result,
            embeddings: mem::take(&mut self.logs)
                .into_iter()
                .map(Embedding::from_log)
                .collect(),
        };
        let el = self.mut_or_insert_element(feature, rule, scenario, ty);
        el.steps.push(step);
    }

    /// Inserts the given `scenario`, if not present, and then returns a mutable
    /// reference to the contained value.
    pub fn mut_or_insert_element(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> &mut Element {
        let f_pos = self
            .features
            .iter()
            .position(|f| f == feature)
            .unwrap_or_else(|| {
                self.features.push(Feature::new(feature));
                self.features.len() - 1
            });
        let f = self
            .features
            .get_mut(f_pos)
            .unwrap_or_else(|| unreachable!());

        let el_pos = f
            .elements
            .iter()
            .position(|el| el.matches_scenario(rule, scenario, ty))
            .unwrap_or_else(|| {
                f.elements.push(Element::new(feature, rule, scenario, ty));
                f.elements.len() - 1
            });
        f.elements
            .get_mut(el_pos)
            .unwrap_or_else(|| unreachable!())
    }

    /// Returns a reference to the features.
    pub fn features(&self) -> &[Feature] {
        &self.features
    }

    /// Returns the current statistics.
    pub fn stats(&self) -> &WriterStats {
        &self.stats
    }

    /// Clears the current logs.
    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    /// Returns whether there are any pending logs.
    pub fn has_logs(&self) -> bool {
        !self.logs.is_empty()
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Hook, HookType, Step};
    use gherkin::{Feature as GherkinFeature, LineCol, Scenario as GherkinScenario};
    use std::{path::PathBuf, time::SystemTime};

    fn create_test_feature() -> GherkinFeature {
        GherkinFeature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            position: LineCol { line: 1, col: 1 },
            path: Some(PathBuf::from("test.feature")),
        }
    }

    fn create_test_scenario() -> GherkinScenario {
        GherkinScenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            tags: vec![],
            position: LineCol { line: 5, col: 1 },
            steps: vec![],
            examples: vec![],
        }
    }

    fn create_test_step() -> gherkin::Step {
        gherkin::Step {
            keyword: "Given".to_string(),
            value: "a test step".to_string(),
            docstring: None,
            table: None,
            position: LineCol { line: 6, col: 1 },
        }
    }

    #[test]
    fn event_handler_new() {
        let handler = EventHandler::new();
        
        assert!(handler.features.is_empty());
        assert!(handler.started.is_none());
        assert!(handler.logs.is_empty());
    }

    #[test]
    fn event_handler_default() {
        let handler = EventHandler::default();
        assert!(handler.features.is_empty());
    }

    #[test]
    fn handle_scenario_log() {
        let mut handler = EventHandler::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = Metadata { at: SystemTime::now() };
        
        handler.handle_scenario_event(
            &feature,
            None,
            &scenario,
            Scenario::Log("Test log message".to_string()),
            meta,
        );
        
        assert_eq!(handler.logs.len(), 1);
        assert_eq!(handler.logs[0], "Test log message");
        assert!(handler.has_logs());
    }

    #[test]
    fn handle_scenario_finished_clears_logs() {
        let mut handler = EventHandler::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = Metadata { at: SystemTime::now() };
        
        handler.logs.push("Test log".to_string());
        
        handler.handle_scenario_event(
            &feature,
            None,
            &scenario,
            Scenario::Finished,
            meta,
        );
        
        assert!(handler.logs.is_empty());
        assert!(!handler.has_logs());
    }

    #[test]
    fn handle_hook_started() {
        let mut handler = EventHandler::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let meta = Metadata { at: SystemTime::now() };
        
        handler.handle_hook_event(
            &feature,
            None,
            &scenario,
            HookType::Before,
            Hook::Started,
            meta,
        );
        
        assert!(handler.started.is_some());
        assert_eq!(handler.started.unwrap(), meta.at);
    }

    #[test]
    fn handle_hook_passed() {
        let mut handler = EventHandler::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let start_time = SystemTime::now();
        let end_time = start_time + std::time::Duration::from_millis(100);
        
        handler.started = Some(start_time);
        handler.logs.push("Hook log".to_string());
        
        handler.handle_hook_event(
            &feature,
            None,
            &scenario,
            HookType::Before,
            Hook::Passed,
            Metadata { at: end_time },
        );
        
        assert_eq!(handler.features.len(), 1);
        assert_eq!(handler.features[0].elements.len(), 1);
        assert_eq!(handler.features[0].elements[0].before.len(), 1);
        
        let hook_result = &handler.features[0].elements[0].before[0];
        assert_eq!(hook_result.result.status, Status::Passed);
        assert!(hook_result.result.duration > 0);
        assert!(hook_result.result.error_message.is_none());
        assert_eq!(hook_result.embeddings.len(), 1);
    }

    #[test]
    fn mut_or_insert_element_creates_feature() {
        let mut handler = EventHandler::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        
        let element = handler.mut_or_insert_element(&feature, None, &scenario, "scenario");
        assert_eq!(element.name, "Test Scenario");
        assert_eq!(element.r#type, "scenario");
        
        assert_eq!(handler.features.len(), 1);
        assert_eq!(handler.features[0].elements.len(), 1);
    }

    #[test]
    fn mut_or_insert_element_reuses_existing() {
        let mut handler = EventHandler::new();
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        
        // First call creates the element
        let element1 = handler.mut_or_insert_element(&feature, None, &scenario, "scenario");
        element1.name = "Modified".to_string();
        
        // Second call should return the same element
        let element2 = handler.mut_or_insert_element(&feature, None, &scenario, "scenario");
        assert_eq!(element2.name, "Modified");
        
        assert_eq!(handler.features.len(), 1);
        assert_eq!(handler.features[0].elements.len(), 1);
    }

    #[test]
    fn clear_logs() {
        let mut handler = EventHandler::new();
        handler.logs.push("test".to_string());
        
        handler.clear_logs();
        
        assert!(handler.logs.is_empty());
        assert!(!handler.has_logs());
    }

    #[test]
    fn features_accessor() {
        let mut handler = EventHandler::new();
        let feature = create_test_feature();
        handler.features.push(Feature::new(&feature));
        
        let features = handler.features();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].name, "Test Feature");
    }

    #[test] 
    fn stats_accessor() {
        let mut handler = EventHandler::new();
        handler.stats.record_passed_step();
        
        let stats = handler.stats();
        assert_eq!(stats.passed_steps(), 1);
    }
}