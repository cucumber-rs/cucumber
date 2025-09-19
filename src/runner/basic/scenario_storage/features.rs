//! Features storage and management for scenario execution.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{
    channel::mpsc,
    stream, Stream, StreamExt as _, TryStreamExt as _,
};

use crate::{
    Event, World,
    event::{self, source::Source, Retries},
    parser,
};

use super::super::{
    cli_and_types::{RetryOptions, ScenarioType, WhichScenarioFn, RetryOptionsFn},
    supporting_structures::ScenarioId,
};

use super::{
    finished::FinishedRulesAndFeatures,
    queue::{ScenarioQueue, RuleScenarios},
};

/// Scenario storage and queue management.
///
/// Manages [`Scenario`]s queues and provides an API for inserting new
/// [`Scenario`]s and picking next [`Scenario`] to execute.
///
/// [`Scenario`]: gherkin::Scenario
pub struct Features {
    /// All [`gherkin::Feature`]s with their [`Scenario`]s to run.
    features: Arc<Mutex<ScenarioQueue>>,
    
    /// Sender for notifying about finished [`Scenario`]s.
    finished_sender: mpsc::UnboundedSender<(
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        bool,
    )>,
    
    /// [`Scenario`]s finishing notification [`Stream`].
    finished_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<(
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        bool,
    )>>>>,
}

impl Default for Features {
    fn default() -> Self {
        let (finished_sender, finished_receiver) = mpsc::unbounded();
        
        Self {
            features: Arc::new(Mutex::new(ScenarioQueue::new())),
            finished_sender,
            finished_receiver: Arc::new(Mutex::new(Some(finished_receiver))),
        }
    }
}

impl Features {
    /// Inserts [`gherkin::Feature`] for execution.
    ///
    /// If [`Scenario`] matches `which_scenario` predicate, then it will be
    /// executed.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub async fn insert<Which>(
        &self,
        feature: gherkin::Feature,
        which_scenario: &Which,
        retry: &RetryOptionsFn,
        cli: &impl parser::CliLike,
    ) -> parser::Result<()>
    where
        Which: ?Sized
            + Fn(
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &impl parser::CliLike,
            ) -> ScenarioType
            + 'static,
    {
        // Implementation would go here
        // This is a placeholder for the actual insert logic
        todo!("Implement feature insertion logic")
    }

    /// Inserts a retried [`Scenario`] for execution.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub async fn insert_retried_scenario(
        &self,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        retries: Option<Retries>,
        retry_options: Option<RetryOptions>,
    ) {
        // Implementation would go here
        todo!("Implement retried scenario insertion")
    }

    /// Returns next [`Scenario`]s to be executed and a [`Stream`] of all
    /// currently executing [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub async fn get(
        &self,
        max_concurrent_scenarios: Option<usize>,
    ) -> (
        Vec<(
            ScenarioId,
            Source<gherkin::Feature>,
            Option<Source<gherkin::Rule>>,
            Source<gherkin::Scenario>,
            ScenarioType,
            Option<Retries>,
        )>,
        impl Stream<
            Item = parser::Result<(
                Source<gherkin::Feature>,
                Option<Source<gherkin::Rule>>,
                event::Cucumber<W>,
            )>,
        >,
        Option<Duration>,
    )
    where
        W: World,
    {
        // Implementation would go here
        todo!("Implement scenario retrieval logic")
    }

    /// Finishes inserting [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub fn finish(&self) {
        // Implementation would go here
        self.finished_sender.close_channel();
    }

    /// Indicates whether this [`Features`] is finished inserting
    /// [`Scenario`]s and all currently executing [`Scenario`]s are finished.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub async fn is_finished(&self, fail_fast: bool) -> bool {
        // Implementation would go here
        todo!("Implement finished check logic")
    }
}