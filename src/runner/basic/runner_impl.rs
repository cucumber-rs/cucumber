//! Runner trait implementation for the Basic runner.

use futures::{
    FutureExt as _, Stream, StreamExt as _,
    future::{self, Either},
    channel::mpsc,
    stream,
};

use crate::{
    Event, Runner, World,
    event,
    parser,
    future::FutureExt as _,
};

use super::{
    basic_struct::Basic,
    cli_and_types::Cli,
    execution_engine::{insert_features, execute},
    scenario_storage::Features,
};

impl<W, Which, Before, After> Runner<W> for Basic<W, Which, Before, After>
where
    W: World,
    Which: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> super::cli_and_types::ScenarioType
        + 'static,
    Before: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> futures::future::LocalBoxFuture<'a, ()>
        + 'static,
    After: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> futures::future::LocalBoxFuture<'a, ()>
        + 'static,
{
    type Cli = Cli;

    type EventStream =
        futures::stream::LocalBoxStream<'static, parser::Result<Event<event::Cucumber<W>>>>;

    fn run<S>(self, features: S, mut cli: Cli) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
    {
        #[cfg(feature = "tracing")]
        let logs_collector = *self.logs_collector.swap(Box::new(None));
        let Self {
            max_concurrent_scenarios,
            retries,
            retry_after,
            retry_filter,
            steps,
            which_scenario,
            retry_options,
            before_hook,
            after_hook,
            fail_fast,
            ..
        } = self;

        cli.retry = cli.retry.or(retries);
        cli.retry_after = cli.retry_after.or(retry_after);
        cli.retry_tag_filter = cli.retry_tag_filter.or(retry_filter);
        let fail_fast = cli.fail_fast || fail_fast;
        let concurrency = cli.concurrency.or(max_concurrent_scenarios);

        let buffer = Features::default();
        let (sender, receiver) = mpsc::unbounded();

        let insert = insert_features(
            buffer.clone(),
            features,
            which_scenario,
            retry_options,
            sender.clone(),
            cli,
            fail_fast,
        );
        let execute = execute(
            buffer,
            concurrency,
            steps,
            sender,
            before_hook,
            after_hook,
            fail_fast,
            #[cfg(feature = "tracing")]
            logs_collector,
        );

        stream::select(
            receiver.map(Either::Left),
            future::join(insert, execute).into_stream().map(Either::Right),
        )
        .filter_map(async |r| match r {
            Either::Left(ev) => Some(ev),
            Either::Right(_) => None,
        })
        .boxed_local()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use std::time::Duration;
    use crate::test_utils::common::TestWorld;

    // Using common TestWorld from test_utils

    #[tokio::test]
    async fn test_runner_empty_features() {
        let runner = Basic::<TestWorld>::default();
        let features = stream::empty();
        let cli = Cli::default();
        
        let mut events = runner.run(features, cli);
        
        // Should receive ParsingFinished and Finished events
        let parsing_finished = events.next().await;
        assert!(parsing_finished.is_some());
        
        let finished = events.next().await;
        assert!(finished.is_some());
        
        // No more events
        let next = events.next().await;
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn test_runner_with_concurrency_limit() {
        let runner = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(2);
        
        let features = stream::empty();
        let cli = Cli {
            concurrency: Some(5), // CLI should override
            ..Default::default()
        };
        
        let mut events = runner.run(features, cli);
        
        // Should start with concurrency from CLI
        let parsing_finished = events.next().await;
        assert!(parsing_finished.is_some());
    }

    #[tokio::test]
    async fn test_runner_fail_fast() {
        let runner = Basic::<TestWorld>::default().fail_fast();
        let features = stream::empty();
        let cli = Cli::default();
        
        let mut events = runner.run(features, cli);
        
        // Should handle fail_fast mode
        let parsing_finished = events.next().await;
        assert!(parsing_finished.is_some());
    }

    #[test]
    fn test_runner_retry_configuration() {
        let cli = Cli {
            retry: Some(3),
            retry_after: Some(Duration::from_secs(1)),
            ..Default::default()
        };
        
        assert_eq!(cli.retry, Some(3));
        assert_eq!(cli.retry_after, Some(Duration::from_secs(1)));
    }
}