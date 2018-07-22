pub mod default;

use std::path::Path;

use gherkin;
use TestResult;

pub trait OutputVisitor : Default {
    fn visit_start(&mut self);
    fn visit_feature(&mut self, feature: &gherkin::Feature, path: &Path);
    fn visit_feature_end(&mut self, feature: &gherkin::Feature);
    fn visit_feature_error<'a>(&mut self, path: &Path, error: &gherkin::Error<'a>);
    fn visit_scenario(&mut self, scenario: &gherkin::Scenario);
    fn visit_scenario_end(&mut self, scenario: &gherkin::Scenario);
    fn visit_scenario_skipped(&mut self, scenario: &gherkin::Scenario);
    fn visit_step(&mut self, scenario: &gherkin::Scenario, step: &gherkin::Step);
    fn visit_step_result(&mut self, scenario: &gherkin::Scenario, step: &gherkin::Step, result: &TestResult);
    fn visit_finish(&mut self);
}
