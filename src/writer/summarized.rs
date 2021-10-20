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

use std::{array, borrow::Cow, collections::HashMap, sync::Arc};

use async_trait::async_trait;
use derive_more::Deref;
use itertools::Itertools as _;

use crate::{
    event, parser, writer::term::Styles, ArbitraryWriter, FailureWriter, World,
    Writer,
};

/// Execution statistics.
///
/// [`Step`]: gherkin::Step
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    /// Number of passed [`Step`]s (or [`Scenario`]s).
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub passed: usize,

    /// Number of skipped [`Step`]s (or [`Scenario`]s).
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub skipped: usize,

    /// Number of failed [`Step`]s (or [`Scenario`]s).
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    pub failed: usize,
}

impl Stats {
    /// Returns total number of [`Step`]s (or [`Scenario`]s), these [`Stats`]
    /// have been collected for.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub const fn total(&self) -> usize {
        self.passed + self.skipped + self.failed
    }
}

/// Alias for [`fn`] used to determine should [`Skipped`] test considered as
/// [`Failed`] or not.
///
/// [`Failed`]: event::Step::Failed
/// [`Skipped`]: event::Step::Skipped
pub type SkipFn =
    fn(&gherkin::Feature, Option<&gherkin::Rule>, &gherkin::Scenario) -> bool;

/// Indicator of a [`Failed`] or [`Skipped`] [`Scenario`].
///
/// [`Failed`]: event::Step::Failed
/// [`Scenario`]: gherkin::Scenario
/// [`Skipped`]: event::Step::Skipped
#[derive(Clone, Copy, Debug)]
enum Indicator {
    /// [`Failed`] [`Scenario`].
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    Failed,

    /// [`Skipped`] [`Scenario`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    Skipped,
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

    /// Number of failed [`Scenario`] hooks.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub failed_hooks: usize,

    /// Handled [`Scenario`]s to collect [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    handled_scenarios: HashMap<Arc<gherkin::Scenario>, Indicator>,
}

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

impl<W, Wr> FailureWriter<W> for Summarized<Wr>
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

    fn hook_errors(&self) -> usize {
        self.failed_hooks
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
            failed_hooks: 0,
            handled_scenarios: HashMap::new(),
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
        use self::{
            event::Step,
            Indicator::{Failed, Skipped},
        };

        match ev {
            Step::Started => {}
            Step::Passed(_) => self.steps.passed += 1,
            Step::Skipped => {
                self.steps.skipped += 1;
                self.scenarios.skipped += 1;
                let _ = self
                    .handled_scenarios
                    .insert(Arc::clone(scenario), Skipped);
            }
            Step::Failed(..) => {
                self.steps.failed += 1;
                self.scenarios.failed += 1;
                let _ =
                    self.handled_scenarios.insert(Arc::clone(scenario), Failed);
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
        use event::{Hook, Scenario};

        match ev {
            Scenario::Started
            | Scenario::Hook(_, Hook::Passed | Hook::Started) => {}
            Scenario::Hook(_, Hook::Failed(..)) => {
                // - If Scenario's last Step failed and then After Hook failed
                //   too, we don't need to track second failure;
                // - If Scenario's last Step was skipped and then After Hook
                //   failed, we need to override skipped Scenario with failed;
                // - If Scenario executed no Steps and then Hook failed, we
                //   track Scenario as failed.
                match self.handled_scenarios.get(scenario) {
                    Some(Indicator::Failed) => {}
                    Some(Indicator::Skipped) => {
                        self.scenarios.skipped -= 1;
                        self.scenarios.failed += 1;
                    }
                    None => {
                        self.scenarios.failed += 1;
                        let _ = self
                            .handled_scenarios
                            .insert(Arc::clone(scenario), Indicator::Failed);
                    }
                }
                self.failed_hooks += 1;
            }
            Scenario::Background(_, ev) | Scenario::Step(_, ev) => {
                self.handle_step(scenario, ev);
            }
            Scenario::Finished => {
                if self.handled_scenarios.remove(scenario).is_none() {
                    self.scenarios.passed += 1;
                }
            }
        }
    }
}

impl<Writer> Summarized<Writer> {
    /// Wraps the given [`Writer`] into a new [`Summarized`] one.
    #[must_use]
    pub fn new(writer: Writer) -> Self {
        Self::from(writer)
    }
}

// We better keep this here, as it's related to summarization only.
#[allow(clippy::multiple_inherent_impl)]
impl Styles {
    /// Generates a formatted summary [`String`].
    #[must_use]
    pub fn summary<W>(&self, summary: &Summarized<W>) -> String {
        let features = self.maybe_plural("feature", summary.features);

        let rules = (summary.rules > 0)
            .then(|| format!("{}\n", self.maybe_plural("rule", summary.rules)))
            .unwrap_or_default();

        let scenarios =
            self.maybe_plural("scenario", summary.scenarios.total());
        let scenarios_stats = self.format_stats(summary.scenarios);

        let steps = self.maybe_plural("step", summary.steps.total());
        let steps_stats = self.format_stats(summary.steps);

        let parsing_errors = (summary.parsing_errors > 0)
            .then(|| {
                self.err(
                    self.maybe_plural("parsing error", summary.parsing_errors),
                )
            })
            .unwrap_or_default();

        let hook_errors = (summary.failed_hooks > 0)
            .then(|| {
                self.err(self.maybe_plural("hook error", summary.failed_hooks))
            })
            .unwrap_or_default();

        let comma = (!parsing_errors.is_empty() && !hook_errors.is_empty())
            .then(|| self.err(", "))
            .unwrap_or_default();

        format!(
            "{}\n{}\n{}{}{}\n{}{}\n{}{}{}",
            self.bold(self.header("[Summary]")),
            features,
            rules,
            scenarios,
            scenarios_stats,
            steps,
            steps_stats,
            parsing_errors,
            comma,
            hook_errors
        )
        .trim_end_matches('\n')
        .to_owned()
    }

    /// Formats [`Stats`] for a terminal output.
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

    /// Adds `s` to `singular` if the given `num` is not `1`.
    fn maybe_plural(
        &self,
        singular: impl Into<Cow<'static, str>>,
        num: usize,
    ) -> Cow<'static, str> {
        self.bold(format!(
            "{} {}{}",
            num,
            singular.into(),
            (num != 1).then(|| "s").unwrap_or_default(),
        ))
    }
}
