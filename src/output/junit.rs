// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use junit_report::{Duration, Report, TestCase, TestSuite};

use crate::event::{FeatureEvent, ScenarioEvent, StepEvent, CucumberEvent};
use crate::{
    EventHandler,
};
use gherkin::{Scenario, Step};
use std::io::stdout;
use std::rc::Rc;

pub struct JunitOutput {
    report: Report,
    test_suite: TestSuite,
}

impl Default for JunitOutput {
    fn default() -> JunitOutput {
        JunitOutput {
            report: Report::new(),
            test_suite: TestSuite::new("default"),
        }
    }
}

impl JunitOutput {
    fn handle_step(&mut self, step: Rc<Step>, _event: StepEvent) {
        let test_case = TestCase::success(
            step.value.as_ref(),
            Duration::seconds(5),
            Some(step.value.to_string()),
        );
        self.test_suite.add_testcase(test_case);
    }

    fn handle_scenario(&mut self, scenario: Rc<Scenario>, event: ScenarioEvent) {
        match event {
            ScenarioEvent::Starting => {
                self.test_suite = TestSuite::new(scenario.name.as_ref());
            },
            ScenarioEvent::Background(step, event) => {
                self.handle_step(step, event);
            },
            ScenarioEvent::Step(step, event) => {
                self.handle_step(step, event);
            },
            ScenarioEvent::Skipped => {
                self.report.add_testsuite(self.test_suite.clone());
            },
            ScenarioEvent::Passed => {
                self.report.add_testsuite(self.test_suite.clone());
            },
            ScenarioEvent::Failed => {
                self.report.add_testsuite(self.test_suite.clone());
            },
        }
    }
}

impl EventHandler for JunitOutput {
    fn handle_event(&mut self, event: CucumberEvent) {
        match event {
            CucumberEvent::Starting => {
            }
            CucumberEvent::Feature(_feature, event) => match event {
                FeatureEvent::Starting => {
                    self.report = Report::new();
                }
                FeatureEvent::Scenario(scenario, event) => {
                    self.handle_scenario(scenario, event)
                }
                FeatureEvent::Rule(_rule, _event) => {}
                FeatureEvent::Finished => {}
            },
            CucumberEvent::Finished => {
                self.report.write_xml(stdout()).unwrap();
            }
        }
    }
}
