use std::convert::Infallible;

use async_trait::async_trait;
use cucumber::{cli, event, given, parser, Event, WorldInit};

#[derive(Clone, Copy, Debug, WorldInit, Default)]
struct World(usize);

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World::default())
    }
}

#[derive(Default)]
struct DebugWriter(usize);

#[async_trait(?Send)]
impl<W: 'static> cucumber::Writer<W> for DebugWriter {
    type Cli = cli::Empty;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        match ev {
            Ok(Event { value, .. }) => match value {
                event::Cucumber::Feature(feature, ev) => match ev {
                    event::Feature::Started => {
                        println!("{}: {}", feature.keyword, feature.name)
                    }
                    event::Feature::Scenario(_scenario, ev) => match ev {
                        event::Scenario::Step(_step, ev) => match ev {
                            event::Step::Failed(_, _, err) => {
                                println!("failed: {err}");
                                self.0 += 1;
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            Err(e) => println!("Error: {e}"),
        }
    }
}

#[given("a failing step")]
async fn failing_scenario_step(_world: &mut World) {
    panic!("Failing step for test purpose");
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use cucumber::WriterExt;

    async fn failures(tag: &'static str) -> usize {
        World::cucumber()
            .with_writer(DebugWriter::default().normalized())
            .filter_run("tests/features/retry", move |_, _, sc| {
                sc.tags.iter().any(|t| t == tag)
            })
            .await
            .0
    }

    // #[tokio::test]
    // async fn untagged_feature_scenarios_should_not_be_retried() {
    //     assert_eq!(failures("no_retry").await, 1, "1 failure is expected");
    // }

    // #[tokio::test]
    // async fn explicit_retry_zero_should_not_retry() {
    //     assert_eq!(
    //         failures("retry-explicit-0").await,
    //         1,
    //         "1 failure is expected"
    //     );
    // }

    #[tokio::test]
    // TDD : is failing since we didn't implement the retry for now
    async fn explicit_retry_one_should_be_retried_two_times() {
        assert_eq!(
            failures("retry-explicit-1").await,
            2,
            "2 failures are expected due to the retry mechanism"
        );
    }
}
