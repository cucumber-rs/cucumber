//! Hook functionality for Cucumber executor.

use std::marker::PhantomData;

use futures::future::LocalBoxFuture;

use crate::{
    Parser, World, Writer, event,
    runner,
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
        ) -> LocalBoxFuture<'a, ()>
        + 'static,
    A: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>
        + 'static,
{
    /// Sets a hook, executed on each [`Scenario`] before running all its
    /// [`Step`]s, including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn before<Before>(
        self,
        func: Before,
    ) -> Cucumber<W, P, I, runner::Basic<W, F, Before, A>, Wr, Cli>
    where
        Before: for<'a> Fn(
                &'a gherkin::Feature,
                Option<&'a gherkin::Rule>,
                &'a gherkin::Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let Self { parser, runner, writer, cli, .. } = self;
        Cucumber {
            parser,
            runner: runner.before(func),
            writer,
            cli,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Sets a hook, executed on each [`Scenario`] after running all its
    /// [`Step`]s, even after [`Skipped`] of [`Failed`] [`Step`]s.
    ///
    /// Last `World` argument is supplied to the function, in case it was
    /// initialized before by running [`before`] hook or any [`Step`].
    ///
    /// [`before`]: Self::before()
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn after<After>(
        self,
        func: After,
    ) -> Cucumber<W, P, I, runner::Basic<W, F, B, After>, Wr, Cli>
    where
        After: for<'a> Fn(
                &'a gherkin::Feature,
                Option<&'a gherkin::Rule>,
                &'a gherkin::Scenario,
                &'a event::ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let Self { parser, runner, writer, cli, .. } = self;
        Cucumber {
            parser,
            runner: runner.after(func),
            writer,
            cli,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}