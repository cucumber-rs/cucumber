// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
//! Key occurrences in the lifecycle of a Cucumber execution.
//!
//! The top-level enum here is `CucumberEvent`.
//!
//! Each event enum contains variants indicating
//! what stage of execution Cucumber is at and,
//! variants with detailed content about the precise
//! sub-event

pub use super::ExampleValues;
use std::{fmt::Display, rc::Rc};

/// The stringified content of stdout and stderr
/// captured during Step execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedOutput {
    pub out: String,
    pub err: String,
}

/// Panic source location information
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

/// Panic content captured when a Step failed.
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

/// Outcome of step execution, carrying along the relevant
/// `World` state.
pub(crate) enum TestEvent<W> {
    Unimplemented,
    Skipped,
    Success(W, CapturedOutput),
    Failure(PanicInfo, CapturedOutput),
    TimedOut,
}

/// Event specific to a particular [Step](https://cucumber.io/docs/gherkin/reference/#step)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepEvent {
    Starting,
    Unimplemented,
    Skipped,
    Passed(CapturedOutput),
    Failed(CapturedOutput, PanicInfo),
    TimedOut,
}

/// Event specific to a particular [Scenario](https://cucumber.io/docs/gherkin/reference/#example)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScenarioEvent {
    Starting(ExampleValues),
    Background(Rc<gherkin::Step>, StepEvent),
    Step(Rc<gherkin::Step>, StepEvent),
    Skipped,
    Passed,
    Failed,
    TimedOut,
}

/// Event specific to a particular [Rule](https://cucumber.io/docs/gherkin/reference/#rule)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleEvent {
    Starting,
    Scenario(Rc<gherkin::Scenario>, ScenarioEvent),
    Skipped,
    Passed,
    Failed,
    TimedOut,
}

/// Event specific to a particular [Feature](https://cucumber.io/docs/gherkin/reference/#feature)
#[derive(Debug, Clone)]
pub enum FeatureEvent {
    Starting,
    Scenario(Rc<gherkin::Scenario>, ScenarioEvent),
    Rule(Rc<gherkin::Rule>, RuleEvent),
    Finished,
}

/// Top-level cucumber run event.
#[derive(Debug, Clone)]
pub enum CucumberEvent {
    Starting,
    Feature(Rc<gherkin::Feature>, FeatureEvent),
    Finished,
}
