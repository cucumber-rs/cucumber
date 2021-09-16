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

use std::{array, borrow::Cow, collections::HashSet, sync::Arc};

use async_trait::async_trait;
use derive_more::Deref;
use itertools::Itertools as _;

use crate::{
    event, parser, writer::term::Styles, ArbitraryWriter, FallibleWriter,
    World, Writer,
};

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

impl Stats {
    /// Returns number of [`Step`]s, [`Stats`] has been collected for.
    ///
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn count(&self) -> usize {
        self.passed + self.skipped + self.failed
    }
}

/// Wrapper for a [`Writer`] for outputting an execution summary (number of
/// executed features, scenarios, steps and parsing errors).
///
/// __Note:__ The underlying [`Writer`] is expected to be an [`ArbitraryWriter`]
/// with `Value` accepting [`String`]. If your underlying [`ArbitraryWriter`]
/// operates with something like JSON (or any other type), you should implement
/// a [`Writer`] on [`Summarized`] by yourself, to provide the required summary
/// format.
#[derive(Debug, Deref)]
pub struct Summarized<Writer> {
    /// Original [`Writer`] to summarize output of.
    #[deref]
    pub writer: Writer,

    /// Number of started [`Feature`]s.
    ///
    /// [`Feature`]: gherkin::Feature
    pub features: usize,

    /// Number of started [`Rule`]s.
    ///
    /// [`Rule`]: gherkin::Rule
    pub rules: usize,

    /// [`Scenario`]s [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub scenarios: Stats,

    /// [`Step`]s [`Stats`].
    ///
    /// [`Step`]: gherkin::Step
    pub steps: Stats,

    /// Number of [`Parser`] errors.
    ///
    /// [`Parser`]: crate::Parser
    pub parsing_errors: usize,

    /// Handled [`Scenario`]s to collect [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    handled_scenarios: HashSet<Arc<gherkin::Scenario>>,
}

/// Alias for [`fn`] used to determine should [`Skipped`] test considered as
/// [`Failed`] or not.
///
/// [`Failed`]: event::Step::Failed
/// [`Skipped`]: event::Step::Skipped
pub type SkipFn =
    fn(&gherkin::Feature, Option<&gherkin::Rule>, &gherkin::Scenario) -> bool;

#[async_trait(?Send)]
impl<W, Wr> Writer<W> for Summarized<Wr>
where
    W: World,
    Wr: for<'val> ArbitraryWriter<'val, W, String>,
{
    async fn handle_event(&mut self, ev: parser::Result<event::Cucumber<W>>) {
        use event::{Cucumber, Feature, Rule};

        let mut finished = false;
        match &ev {
            Err(_) => self.parsing_errors += 1,
            Ok(Cucumber::Feature(_, ev)) => match ev {
                Feature::Started => self.features += 1,
                Feature::Rule(_, Rule::Started) => {
                    self.rules += 1;
                }
                Feature::Rule(_, Rule::Scenario(sc, ev))
                | Feature::Scenario(sc, ev) => {
                    self.handle_scenario(sc, ev);
                }
                Feature::Finished | Feature::Rule(..) => {}
            },
            Ok(Cucumber::Finished) => finished = true,
            Ok(Cucumber::Started) => {}
        };

        self.writer.handle_event(ev).await;

        if finished {
            self.writer.write(Styles::new().summary(self)).await;
        }
    }
}

#[async_trait(?Send)]
impl<'val, W, Wr, Val> ArbitraryWriter<'val, W, Val> for Summarized<Wr>
where
    W: World,
    Self: Writer<W>,
    Wr: ArbitraryWriter<'val, W, Val>,
    Val: 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.writer.write(val).await;
    }
}

impl<W, Wr> FallibleWriter<W> for Summarized<Wr>
where
    W: World,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        self.steps.failed
    }

    fn parsing_errors(&self) -> usize {
        self.parsing_errors
    }
}

impl<Writer> From<Writer> for Summarized<Writer> {
    fn from(writer: Writer) -> Self {
        Self {
            writer,
            features: 0,
            rules: 0,
            scenarios: Stats {
                passed: 0,
                skipped: 0,
                failed: 0,
            },
            steps: Stats {
                passed: 0,
                skipped: 0,
                failed: 0,
            },
            parsing_errors: 0,
            handled_scenarios: HashSet::new(),
        }
    }
}

impl<Writer> Summarized<Writer> {
    /// Keeps track of [`Step`]'s [`Stats`].
    ///
    /// [`Step`]: gherkin::Step
    fn handle_step<W>(
        &mut self,
        scenario: &Arc<gherkin::Scenario>,
        ev: &event::Step<W>,
    ) {
        use event::Step;

        match ev {
            Step::Started => {}
            Step::Passed => self.steps.passed += 1,
            Step::Skipped => {
                self.steps.skipped += 1;
                self.scenarios.skipped += 1;
                let _ = self.handled_scenarios.insert(scenario.clone());
            }
            Step::Failed(..) => {
                self.steps.failed += 1;
                self.scenarios.failed += 1;
                let _ = self.handled_scenarios.insert(scenario.clone());
            }
        }
    }

    /// Keeps track of [`Scenario`]'s [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn handle_scenario<W>(
        &mut self,
        scenario: &Arc<gherkin::Scenario>,
        ev: &event::Scenario<W>,
    ) {
        use event::Scenario;

        match ev {
            Scenario::Started => {}
            Scenario::Background(_, ev) | Scenario::Step(_, ev) => {
                self.handle_step(scenario, ev);
            }
            Scenario::Finished => {
                if !self.handled_scenarios.remove(scenario) {
                    self.scenarios.passed += 1;
                }
            }
        }
    }
}

impl<Writer> Summarized<Writer> {
    /// Creates a new [`Summarized`] [`Writer`].
    #[must_use]
    pub fn new(writer: Writer) -> Summarized<Writer> {
        Summarized::from(writer)
    }
}

impl Styles {
    /// Generates formatted summary [`String`].
    #[must_use]
    pub fn summary<W>(&self, summary: &Summarized<W>) -> String {
        let features = self.maybe_plural("feature", summary.features);

        let rules = (summary.rules > 0)
            .then(|| self.maybe_plural("rule", summary.rules))
            .unwrap_or_default();

        let scenarios =
            self.maybe_plural("scenario", summary.scenarios.count());
        let scenarios_stats = self.format_stats(summary.scenarios);

        let steps = self.maybe_plural("step", summary.steps.count());
        let steps_stats = self.format_stats(summary.steps);

        let parsing_errors = (summary.parsing_errors > 0)
            .then(|| {
                self.err(
                    self.maybe_plural("parsing error", summary.parsing_errors),
                )
            })
            .unwrap_or_default();

        format!(
            "{}\n{}\n{}\n{}{}\n{}{}\n{}",
            self.bold(self.header("[Summary]")),
            features,
            rules,
            scenarios,
            scenarios_stats,
            steps,
            steps_stats,
            parsing_errors,
        )
    }

    /// Formats [`Stats`] for terminal output.
    #[must_use]
    pub fn format_stats(&self, stats: Stats) -> Cow<'static, str> {
        let formatted = array::IntoIter::new([
            (stats.passed > 0)
                .then(|| self.bold(self.ok(format!("{} passed", stats.passed))))
                .unwrap_or_default(),
            (stats.skipped > 0)
                .then(|| {
                    self.bold(
                        self.skipped(format!("{} skipped", stats.skipped)),
                    )
                })
                .unwrap_or_default(),
            (stats.failed > 0)
                .then(|| {
                    self.bold(self.err(format!("{} failed", stats.failed)))
                })
                .unwrap_or_default(),
        ])
        .filter(|s| !s.is_empty())
        .join(&self.bold(", "));

        (!formatted.is_empty())
            .then(|| {
                self.bold(format!(
                    " {}{}{}",
                    self.bold("("),
                    formatted,
                    self.bold(")")
                ))
            })
            .unwrap_or_default()
    }

    /// Adds `s` to `singular`, if `num != 1`
    #[must_use]
    fn maybe_plural(
        &self,
        singular: impl Into<Cow<'static, str>>,
        num: usize,
    ) -> Cow<'static, str> {
        self.bold(format!(
            "{} {}{}",
            num,
            singular.into(),
            (num != 1).then(|| "s").unwrap_or_default()
        ))
    }
}
