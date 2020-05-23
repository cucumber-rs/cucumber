// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{fmt::Display, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedOutput {
    pub out: String,
    pub err: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl Location {
    pub fn unknown() -> Self {
        Location {
            file: "<unknown>".into(),
            line: 0,
            column: 0,
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\u{00a0}{}:{}:{}\u{00a0}",
            &self.file, self.line, self.column
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanicInfo {
    pub location: Location,
    pub payload: String,
}

impl PanicInfo {
    pub fn unknown() -> Self {
        PanicInfo {
            location: Location::unknown(),
            payload: "(No panic info was found?)".into(),
        }
    }
}

pub enum TestEvent<W> {
    Unimplemented,
    Skipped,
    Success(W, CapturedOutput),
    Failure(PanicInfo, CapturedOutput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepEvent {
    Unimplemented,
    Skipped,
    Passed(CapturedOutput),
    Failed(CapturedOutput, PanicInfo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioEvent {
    Starting,
    Background(Rc<gherkin::Step>, StepEvent),
    Step(Rc<gherkin::Step>, StepEvent),
    Skipped,
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleEvent {
    Starting,
    Scenario(Rc<gherkin::Scenario>, ScenarioEvent),
    Skipped,
    Passed,
    Failed,
}

#[derive(Debug, Clone)]
pub enum FeatureEvent {
    Starting,
    Scenario(Rc<gherkin::Scenario>, ScenarioEvent),
    Rule(Rc<gherkin::Rule>, RuleEvent),
    Finished,
}

#[derive(Debug, Clone)]
pub enum CucumberEvent {
    Starting,
    Feature(Rc<gherkin::Feature>, FeatureEvent),
    Finished,
}
