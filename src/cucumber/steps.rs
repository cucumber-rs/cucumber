//! Step definition functionality for Cucumber executor.

use regex::Regex;

use crate::{
    Parser, World, Writer, Step,
    runner, step,
};

use super::core::Cucumber;

impl<W, I, P, Wr, F, B, A, Cli>
    Cucumber<W, P, I, runner::Basic<W, F, B, A>, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
    Cli: clap::Args,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> crate::ScenarioType
        + 'static,
    B: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> futures::future::LocalBoxFuture<'a, ()>
        + 'static,
    A: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a crate::event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> futures::future::LocalBoxFuture<'a, ()>
        + 'static,
{
    /// Replaces [`Collection`] of [`Step`]s.
    ///
    /// [`Collection`]: step::Collection
    /// [`Step`]: step::Step
    #[must_use]
    pub fn steps(mut self, steps: step::Collection<W>) -> Self {
        self.runner = self.runner.steps(steps);
        self
    }

    /// Inserts [Given] [`Step`].
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[must_use]
    pub fn given(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.given(regex, step);
        self
    }

    /// Inserts [When] [`Step`].
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    #[must_use]
    pub fn when(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.when(regex, step);
        self
    }

    /// Inserts [Then] [`Step`].
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    #[must_use]
    pub fn then(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.then(regex, step);
        self
    }
}