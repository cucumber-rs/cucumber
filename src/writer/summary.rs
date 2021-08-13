// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Writer`] for collecting summary.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::{event, World, Writer};

/// [`Writer`] for collecting summary: number of features, scenarios and steps.
#[derive(Debug)]
pub struct Summary<Writer> {
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
impl<W, Wr> Writer<W> for Summary<Wr>
where
    W: World,
    Wr: Writer<W>,
{
    async fn handle_event(&mut self, ev: event::Cucumber<W>) {
        let mut finished = false;
        match &ev {
            event::Cucumber::Feature(_, ev) => match ev {
                event::Feature::Started => self.features += 1,
                event::Feature::Rule(_, event::Rule::Started) => {
                    self.rules += 1;
                }
                event::Feature::Rule(_, event::Rule::Scenario(_, ev))
                | event::Feature::Scenario(_, ev) => self.handle_scenario(ev),
                event::Feature::Finished | event::Feature::Rule(..) => {}
            },
            event::Cucumber::Finished => finished = true,
            event::Cucumber::Started => {}
        };

        self.writer.handle_event(ev).await;

        if finished {
            println!(
                "[Summary]\n\
                 {} features\n\
                 {} rules\n\
                 {} scenarios\n\
                 {} steps ({} passed, {} skipped, {} failed)",
                self.features,
                self.rules,
                self.scenarios,
                self.steps.passed + self.steps.skipped + self.steps.failed,
                self.steps.passed,
                self.steps.skipped,
                self.steps.failed,
            );
        }
    }
}

impl<Writer> Summary<Writer> {
    /// Creates new [`Summary`].
    pub fn new(writer: Writer) -> Self {
        Self {
            writer,
            features: 0,
            rules: 0,
            scenarios: 0,
            steps: Stats {
                passed: 0,
                skipped: 0,
                failed: 0,
            },
        }
    }

    /// Indicates whether or not there have been failed [`Step`]s.
    ///
    /// [`Step`]: gherkin::Step
    pub fn is_failed(&self) -> bool {
        self.steps.failed > 0
    }

    fn handle_step<W>(&mut self, ev: &event::Step<W>) {
        match ev {
            event::Step::Started => {}
            event::Step::Passed => self.steps.passed += 1,
            event::Step::Skipped => self.steps.skipped += 1,
            event::Step::Failed(..) => self.steps.failed += 1,
        }
    }

    fn handle_scenario<W>(&mut self, ev: &event::Scenario<W>) {
        match ev {
            event::Scenario::Started => self.scenarios += 1,
            event::Scenario::Background(_, ev)
            | event::Scenario::Step(_, ev) => self.handle_step(ev),
            event::Scenario::Finished => {}
        }
    }
}
