use std;
use std::collections::HashMap;
use std::default::Default;
use std::env;
use std::io::Write;
use std::path::Path;

use gherkin;
use pathdiff::diff_paths;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use textwrap;

use crate::OutputVisitor;
use crate::TestResult;

pub struct DebugOutput;

impl OutputVisitor for DebugOutput {
    fn new() -> Self
    where
        Self: Sized,
    {
        DebugOutput
    }

    fn visit_start(&mut self) {
        println!("visit_start");
    }

    fn visit_feature(&mut self, feature: &gherkin::Feature, path: &Path) {
        println!("visit_feature {} {}", feature.name, path.display());
    }

    fn visit_feature_end(&mut self, feature: &gherkin::Feature) {
        println!("visit_feature_end {}", feature.name);
    }

    fn visit_feature_error(&mut self, path: &Path, error: &gherkin::Error) {
        println!("visit_feature_error {} {}", path.display(), error);
    }

    fn visit_rule(&mut self, rule: &gherkin::Rule) {
        println!("visit_rule {}", rule.name);
    }

    fn visit_rule_end(&mut self, rule: &gherkin::Rule) {
        println!("visit_rule_end {}", rule.name);
    }

    fn visit_scenario(&mut self, rule: Option<&gherkin::Rule>, scenario: &crate::Scenario) {
        println!("visit_scenario {}", scenario.name);
    }

    fn visit_scenario_end(&mut self, rule: Option<&gherkin::Rule>, scenario: &crate::Scenario) {
        println!("visit_scenario_end {}", scenario.name);
    }

    fn visit_scenario_skipped(&mut self, rule: Option<&gherkin::Rule>, scenario: &crate::Scenario) {
        println!("visit_scenario_skipped {}", scenario.name);
    }

    fn visit_step(
        &mut self,
        rule: Option<&gherkin::Rule>,
        scenario: &crate::Scenario,
        step: &crate::Step,
    ) {
        println!("visit_step {} {}", step.raw_type, step.value);
    }

    fn visit_step_result(
        &mut self,
        rule: Option<&gherkin::Rule>,
        scenario: &crate::Scenario,
        step: &crate::Step,
        result: &TestResult,
    ) {
        println!(
            "visit_step_result {} {} - {:?}",
            step.raw_type, step.value, result
        );
    }

    fn visit_finish(&mut self) {
        println!("visit_finish");
    }

    fn visit_step_resolved<'a, W: crate::World>(
        &mut self,
        step: &crate::Step,
        test: &crate::TestCaseType<'a, W>,
    ) {
        println!("visit_step_resolved {:?}", test);
    }
}
