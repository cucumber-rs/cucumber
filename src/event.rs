// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedOutput {
    pub out: String,
    pub err: String,
}

pub(crate) enum TestEvent<W> {
    Unimplemented,
    Skipped,
    Success(W, CapturedOutput),
    Failure(Option<String>, CapturedOutput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepEvent {
    Unimplemented,
    Skipped,
    Passed(CapturedOutput),
    Failed(CapturedOutput, Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScenarioEvent {
    Starting,
    Background(Rc<gherkin::Step>, StepEvent),
    Step(Rc<gherkin::Step>, StepEvent),
    Skipped,
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuleEvent {
    Starting,
    Scenario(Rc<gherkin::Scenario>, ScenarioEvent),
    Skipped,
    Passed,
    Failed,
}

#[derive(Debug, Clone)]
pub(crate) enum FeatureEvent {
    Starting,
    Scenario(Rc<gherkin::Scenario>, ScenarioEvent),
    Rule(Rc<gherkin::Rule>, RuleEvent),
    Finished,
}

#[derive(Debug, Clone)]
pub(crate) enum CucumberEvent {
    Starting,
    Feature(Rc<gherkin::Feature>, FeatureEvent),
    Finished,
}
