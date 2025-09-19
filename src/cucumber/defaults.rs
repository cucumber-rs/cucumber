//! Default implementations for Cucumber executor.

use std::path::Path;

use derive_more::with_trait::Debug;

use crate::{
    World, runner, parser, writer,
    writer::Ext as _,
};

use super::core::Cucumber;

/// Shortcut for the [`Cucumber`] type returned by its [`Default`] impl.
pub type DefaultCucumber<W, I> = Cucumber<
    W,
    parser::Basic,
    I,
    runner::Basic<W>,
    writer::Summarize<writer::Normalize<W, writer::Basic>>,
>;

impl<W, I> Default for DefaultCucumber<W, I>
where
    W: World + Debug,
    I: AsRef<Path>,
{
    fn default() -> Self {
        Self::custom(
            parser::Basic::new(),
            runner::Basic::default(),
            writer::Basic::stdout().summarized(),
        )
    }
}

impl<W, I> DefaultCucumber<W, I>
where
    W: World + Debug,
    I: AsRef<Path>,
{
    /// Creates a default [`Cucumber`] executor.
    ///
    /// * [`Parser`] — [`parser::Basic`]
    ///
    /// * [`Runner`] — [`runner::Basic`]
    ///   * [`ScenarioType`] — [`Concurrent`] by default, [`Serial`] if
    ///     `@serial` [tag] is present on a [`Scenario`];
    ///   * Allowed to run up to 64 [`Concurrent`] [`Scenario`]s.
    ///
    /// * [`Writer`] — [`Normalize`] and [`Summarize`] [`writer::Basic`].
    ///
    /// [`Concurrent`]: crate::ScenarioType::Concurrent
    /// [`Normalize`]: writer::Normalize
    /// [`Parser`]: crate::Parser
    /// [`Runner`]: crate::Runner
    /// [`Scenario`]: gherkin::Scenario
    /// [`ScenarioType`]: crate::ScenarioType
    /// [`Serial`]: crate::ScenarioType::Serial
    /// [`Summarize`]: writer::Summarize
    ///
    /// [tag]: https://cucumber.io/docs/cucumber/api#tags
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}