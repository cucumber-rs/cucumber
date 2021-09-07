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

use async_trait::async_trait;
use derive_more::Deref;

use crate::{event, parser, ArbitraryWriter, World, Writer};

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

/// Wrapper for a [`Writer`] for outputting an execution summary (number of
/// executed features, scenarios, steps and parsing errors).
#[derive(Debug, Deref)]
pub struct Summarized<Writer, F = SkipFn> {
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

    /// Number of started [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub scenarios: usize,

    /// Number of [`Parser`] errors.
    ///
    /// [`Parser`]: crate::Parser
    pub parsing_errors: usize,

    /// [`Step`]s [`Stats`].
    ///
    /// [`Step`]: gherkin::Step
    pub steps: Stats,

    /// If [`Some`], uses underlying [`Fn`] to determine whether [`Skipped`]
    /// test should be considered as [`Failed`] or not.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Skipped`]: event::Step::Skipped
    fail_on_skip: Option<F>,
}

/// Alias for [`fn`] used to determine should [`Skipped`] test considered as
/// [`Failed`] or not.
///
/// [`Failed`]: event::Step::Failed
/// [`Skipped`]: event::Step::Skipped
pub type SkipFn =
    fn(&gherkin::Feature, Option<&gherkin::Rule>, &gherkin::Scenario) -> bool;

#[async_trait(?Send)]
impl<W, Wr, F> Writer<W> for Summarized<Wr, F>
where
    W: World,
    F: Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
    ) -> bool,
    Wr: for<'val> ArbitraryWriter<'val, W, String>,
{
    async fn handle_event(&mut self, ev: parser::Result<event::Cucumber<W>>) {
        use event::{Cucumber, Feature, Rule};

        let mut finished = false;
        match &ev {
            Err(_) => self.parsing_errors += 1,
            Ok(Cucumber::Feature(f, ev)) => match ev {
                Feature::Started => self.features += 1,
                Feature::Rule(_, Rule::Started) => {
                    self.rules += 1;
                }
                Feature::Rule(r, Rule::Scenario(sc, ev)) => {
                    self.handle_scenario(f, Some(r.as_ref()), sc.as_ref(), ev);
                }
                Feature::Scenario(sc, ev) => {
                    self.handle_scenario(f, None, sc.as_ref(), ev);
                }
                Feature::Finished | Feature::Rule(..) => {}
            },
            Ok(Cucumber::Finished) => finished = true,
            Ok(Cucumber::Started) => {}
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
impl<'val, W, Wr, Val, F> ArbitraryWriter<'val, W, Val> for Summarized<Wr, F>
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

impl<Writer> From<Writer> for Summarized<Writer> {
    fn from(writer: Writer) -> Self {
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
            fail_on_skip: None,
        }
    }
}

impl<Writer, F> Summarized<Writer, F> {
    /// Creates a new [`Summarized`] [`Writer`].
    #[must_use]
    pub fn new(writer: Writer) -> Summarized<Writer> {
        Summarized::from(writer)
    }

    /// Consider [`Skipped`] test as [`Failed`] if [`Scenario`] isn't marked
    /// with `@allow_skipped` tag.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    #[must_use]
    pub fn fail_on_skipped(self) -> Summarized<Writer> {
        Summarized {
            writer: self.writer,
            features: self.features,
            rules: self.rules,
            scenarios: self.scenarios,
            parsing_errors: self.parsing_errors,
            steps: self.steps,
            fail_on_skip: Some(|_, _, sc| {
                !sc.tags.iter().any(|tag| tag == "allow_skipped")
            }),
        }
    }

    /// Consider [`Skipped`] test as [`Failed`] if `Filter` predicate returns
    /// `true`.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    #[must_use]
    pub fn fail_on_skipped_with<Filter>(
        self,
        func: Filter,
    ) -> Summarized<Writer, Filter>
    where
        Filter: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> bool,
    {
        Summarized {
            writer: self.writer,
            features: self.features,
            rules: self.rules,
            scenarios: self.scenarios,
            parsing_errors: self.parsing_errors,
            steps: self.steps,
            fail_on_skip: Some(func),
        }
    }

    /// Indicates whether there have been failed [`Step`]s or [`Parser`] errors.
    ///
    /// [`Parser`]: crate::Parser
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.steps.failed > 0 || self.parsing_errors > 0
    }
}

impl<Writer, F> Summarized<Writer, F>
where
    F: Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
    ) -> bool,
{
    /// Keeps track of [`Step`]'s [`Stats`].
    ///
    /// [`Step`]: gherkin::Step
    fn handle_step<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: &event::Step<W>,
    ) {
        use event::Step;

        match ev {
            Step::Started => {}
            Step::Passed => self.steps.passed += 1,
            Step::Skipped => {
                if self
                    .fail_on_skip
                    .as_ref()
                    .map(|f| f(feature, rule, scenario))
                    .unwrap_or_default()
                {
                    self.steps.failed += 1;
                } else {
                    self.steps.skipped += 1;
                }
            }
            Step::Failed(..) => self.steps.failed += 1,
        }
    }

    /// Keeps track of [`Scenario`]'s [`Stats`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    fn handle_scenario<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: &event::Scenario<W>,
    ) {
        use event::Scenario;

        match ev {
            Scenario::Started => self.scenarios += 1,
            Scenario::Background(_, ev) | Scenario::Step(_, ev) => {
                self.handle_step(feature, rule, scenario, ev);
            }
            Scenario::Finished => {}
        }
    }
}
