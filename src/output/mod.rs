pub mod default;
// pub mod multi;

use std::path::Path;

use gherkin;

use crate::TestResult;

pub trait OutputVisitor: Default {
    fn new() -> Self
    where
        Self: Sized;
    fn visit_start(&self);
    fn visit_feature(&self, feature: &gherkin::Feature, path: &Path);
    fn visit_feature_end(&self, feature: &gherkin::Feature);
    fn visit_feature_error(&self, path: &Path, error: &gherkin::Error);
    fn visit_rule(&self, rule: &gherkin::Rule);
    fn visit_rule_end(&self, rule: &gherkin::Rule);
    fn visit_scenario(&self, rule: Option<&gherkin::Rule>, scenario: &gherkin::Scenario);
    fn visit_scenario_end(&self, rule: Option<&gherkin::Rule>, scenario: &gherkin::Scenario);
    fn visit_scenario_skipped(
        &self,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
    );
    fn visit_step(
        &self,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        step: &gherkin::Step,
    );
    fn visit_step_result(
        &self,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        step: &gherkin::Step,
        result: &TestResult,
    );
    fn visit_finish(&self);
}
