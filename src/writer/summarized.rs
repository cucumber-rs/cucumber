// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Writer`]-wrapper for collecting a summary of execution.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::{event, OutputtedWriter, World, Writer};

/// [`Writer`] for collecting summary: number of features, scenarios and steps.
///
/// Wrapper for a [`Writer`] implementation for outputting a summary (number of
/// features, scenarios, steps and parsing errors) of execution.
#[derive(Debug)]
pub struct Summarized<Writer> {
    writer: Writer,

    /// Number of started [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    pub features: usize,

    /// Number of started [`Rule`]s.
    ///
    /// [`Rule`]: gherkin::Rule
    pub rules: usize,

    /// Number of started [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub scenarios: usize,

    /// Number of [`Parser`] errors.
    ///
    /// [`Parser`]: crate::Parser
    pub parsing_errors: usize,

    /// [`Step`]s [`Stats`]
    ///
    /// [`Step`]: gherkin::Step
    pub steps: Stats,
}

/// [`Step`]s statistics.
///
/// [`Step`]: gherkin::Step
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    /// Number of passed [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub passed: usize,

    /// Number of skipped [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub skipped: usize,

    /// Number of failed [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub failed: usize,
}

#[async_trait(?Send)]
impl<W, Wr> Writer<W> for Summarized<Wr>
where
    W: World,
    Wr: for<'val> OutputtedWriter<'val, W, String>,
{
    async fn handle_event(&mut self, ev: event::Cucumber<W>) {
        use event::{Cucumber, Feature, Rule};

        let mut finished = false;
        match &ev {
            Cucumber::ParsingError(_) => self.parsing_errors += 1,
            Cucumber::Feature(_, ev) => match ev {
                Feature::Started => self.features += 1,
                Feature::Rule(_, Rule::Started) => {
                    self.rules += 1;
                }
                Feature::Rule(_, Rule::Scenario(_, ev))
                | Feature::Scenario(_, ev) => self.handle_scenario(ev),
                Feature::Finished | Feature::Rule(..) => {}
            },
            Cucumber::Finished => finished = true,
            Cucumber::Started => {}
        };

        self.writer.handle_event(ev).await;

        if finished {
            let summary = format!(
                "[Summary]\n\
                 {} features\n\
                 {} rules\n\
                 {} scenarios\n\
                 {} steps ({} passed, {} skipped, {} failed)\n\
                 {} parsing errors",
                self.features,
                self.rules,
                self.scenarios,
                self.steps.passed + self.steps.skipped + self.steps.failed,
                self.steps.passed,
                self.steps.skipped,
                self.steps.failed,
                self.parsing_errors,
            );
            self.writer.write(summary).await;
        }
    }
}

#[async_trait(?Send)]
impl<'val, W, Wr, Output> OutputtedWriter<'val, W, Output> for Summarized<Wr>
where
    W: World,
    Self: Writer<W>,
    Wr: OutputtedWriter<'val, W, Output>,
    Output: 'val,
{
    async fn write(&mut self, val: Output)
    where
        'val: 'async_trait,
    {
        self.writer.write(val).await;
    }
}

impl<Writer> Summarized<Writer> {
    /// Creates new [`Summarized`].
    pub fn new(writer: Writer) -> Self {
        Self {
            writer,
            features: 0,
            rules: 0,
            scenarios: 0,
            parsing_errors: 0,
            steps: Stats {
                passed: 0,
                skipped: 0,
                failed: 0,
            },
        }
    }

    /// Indicates whether or not there have been failed [`Step`]s or [`Parser`]
    /// errors.
    ///
    /// [`Parser`]: crate::Parser
    /// [`Step`]: gherkin::Step
    pub fn is_failed(&self) -> bool {
        self.steps.failed > 0 || self.parsing_errors > 0
    }

    /// Keeps track of [`Step`] [`Stats`].
    ///
    /// [`Step`]: gherkin::Step
    fn handle_step<W>(&mut self, ev: &event::Step<W>) {
        use event::Step;

        match ev {
            Step::Started => {}
            Step::Passed => self.steps.passed += 1,
            Step::Skipped => self.steps.skipped += 1,
            Step::Failed(..) => self.steps.failed += 1,
        }
    }

    /// Keeps track of [`Scenario`] [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn handle_scenario<W>(&mut self, ev: &event::Scenario<W>) {
        use event::Scenario;

        match ev {
            Scenario::Started => self.scenarios += 1,
            Scenario::Background(_, ev) | Scenario::Step(_, ev) => {
                self.handle_step(ev);
            }
            Scenario::Finished => {}
        }
    }
}
