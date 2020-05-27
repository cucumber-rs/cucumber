// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use junit_report::{Duration, Report, TestCase, TestSuite};

use crate::event::FeatureEvent;
use crate::{
    event::{CucumberEvent},
    EventHandler,
};
use std::io::stdout;

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

impl JunitOutput {}

impl EventHandler for JunitOutput {
    fn handle_event(&mut self, event: CucumberEvent) {
        match event {
            CucumberEvent::Starting => {}
            CucumberEvent::Feature(feature, event) => match event {
                FeatureEvent::Starting => self.test_suite = TestSuite::new(feature.name.as_ref()),
                FeatureEvent::Scenario(scenario, _event) => {
                    let test_case = TestCase::success(
                        scenario.name.as_ref(),
                        Duration::seconds(5),
                        Some("you-tell-me".parse().unwrap()),
                    );
                    self.test_suite.add_testcase(test_case);
                }
                FeatureEvent::Rule(_rule, _event) => {}
                FeatureEvent::Finished => {
                    self.report.add_testsuite(self.test_suite.clone());
                    self.report.write_xml(stdout()).unwrap();
                }
            },
            CucumberEvent::Finished => {}
        }
    }
}
