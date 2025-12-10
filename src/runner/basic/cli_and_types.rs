//! CLI options and type definitions for Basic runner.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use futures::future::LocalBoxFuture;
use gherkin::tagexpr::TagOperation;

use crate::{
    event::{self, Retries},
    tag::Ext as _,
};

/// CLI options of a [`Basic`] [`Runner`].
///
/// [`Basic`]: super::Basic
/// [`Runner`]: crate::Runner
#[derive(Clone, Debug, Default, clap::Args)]
#[group(skip)]
pub struct Cli {
    /// Number of scenarios to run concurrently. If not specified, uses the
    /// value configured in tests runner, or 64 by default.
    #[arg(long, short, value_name = "int", global = true)]
    pub concurrency: Option<usize>,

    /// Run tests until the first failure.
    #[arg(long, global = true, visible_alias = "ff")]
    pub fail_fast: bool,

    /// Number of times a scenario will be retried in case of a failure.
    #[arg(long, value_name = "int", global = true)]
    pub retry: Option<usize>,

    /// Delay between each scenario retry attempt.
    ///
    /// Duration is represented in a human-readable format like `12min5s`.
    /// Supported suffixes:
    /// - `nsec`, `ns` — nanoseconds.
    /// - `usec`, `us` — microseconds.
    /// - `msec`, `ms` — milliseconds.
    /// - `seconds`, `second`, `sec`, `s` - seconds.
    /// - `minutes`, `minute`, `min`, `m` - minutes.
    #[arg(
        long,
        value_name = "duration",
        value_parser = humantime::parse_duration,
        verbatim_doc_comment,
        global = true,
    )]
    pub retry_after: Option<Duration>,

    /// Tag expression to filter retried scenarios.
    #[arg(long, value_name = "tagexpr", global = true)]
    pub retry_tag_filter: Option<TagOperation>,
}

/// Type determining whether [`Scenario`]s should run concurrently or
/// sequentially.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScenarioType {
    /// Run [`Scenario`]s sequentially (one-by-one).
    ///
    /// [`Scenario`]: gherkin::Scenario
    Serial,

    /// Run [`Scenario`]s concurrently.
    ///
    /// [`Scenario`]: gherkin::Scenario
    Concurrent,
}

/// Options for retrying [`Scenario`]s.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryOptions {
    /// Number of [`Retries`].
    pub retries: Retries,

    /// Delay before next retry attempt will be executed.
    pub after: Option<Duration>,
}

impl RetryOptions {
    /// Returns [`Some`], in case next retry attempt is available, or [`None`]
    /// otherwise.
    #[must_use]
    pub fn next_try(self) -> Option<Self> {
        self.retries
            .next_try()
            .map(|num| Self { retries: num, after: self.after })
    }

    /// Parses [`RetryOptions`] from [`Feature`]'s, [`Rule`]'s, [`Scenario`]'s
    /// tags and [`Cli`] options.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn parse_from_tags(
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        cli: &Cli,
    ) -> Option<Self> {
        let parse_tags = |tags: &[String]| {
            tags.iter().find_map(|tag| {
                tag.strip_prefix("retry").map(|retries| {
                    let (num, rest) = retries
                        .strip_prefix('(')
                        .and_then(|s| {
                            let (num, rest) = s.split_once(')')?;
                            num.parse::<usize>()
                                .ok()
                                .map(|num| (Some(num), rest))
                        })
                        .unwrap_or((None, retries));

                    let after = rest.strip_prefix(".after").and_then(|after| {
                        let after = after.strip_prefix('(')?;
                        let (dur, _) = after.split_once(')')?;
                        humantime::parse_duration(dur).ok()
                    });

                    (num, after)
                })
            })
        };

        let apply_cli = |options: Option<_>| {
            let matched = cli.retry_tag_filter.as_ref().map_or_else(
                || cli.retry.is_some() || cli.retry_after.is_some(),
                |op| {
                    op.eval(scenario.tags.iter().chain(
                        rule.iter().flat_map(|r| &r.tags).chain(&feature.tags),
                    ))
                },
            );

            (options.is_some() || matched).then(|| Self {
                retries: Retries::initial(
                    options.and_then(|(r, _)| r).or(cli.retry).unwrap_or(1),
                ),
                after: options.and_then(|(_, a)| a).or(cli.retry_after),
            })
        };

        apply_cli(
            parse_tags(&scenario.tags)
                .or_else(|| rule.and_then(|r| parse_tags(&r.tags)))
                .or_else(|| parse_tags(&feature.tags)),
        )
    }

    /// Constructs [`RetryOptionsWithDeadline`], that will reschedule
    /// [`Scenario`] [`after`] delay.
    ///
    /// [`after`]: RetryOptions::after
    /// [`Scenario`]: gherkin::Scenario
    pub(super) fn with_deadline(self, now: Instant) -> RetryOptionsWithDeadline {
        RetryOptionsWithDeadline {
            retries: self.retries,
            after: self.after.map(|at| (at, Some(now))),
        }
    }

    /// Constructs [`RetryOptionsWithDeadline`], that will reschedule
    /// [`Scenario`] immediately, ignoring [`RetryOptions::after`]. Used for
    /// initial [`Scenario`] run, where we don't need to wait for the delay.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(super) fn without_deadline(self) -> RetryOptionsWithDeadline {
        RetryOptionsWithDeadline {
            retries: self.retries,
            after: self.after.map(|at| (at, None)),
        }
    }
}

/// [`RetryOptions`] with an [`Option`]al [`Instant`] to determine, whether
/// [`Scenario`] should be already rescheduled or not.
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug)]
pub struct RetryOptionsWithDeadline {
    /// Number of [`Retries`].
    pub retries: Retries,

    /// Delay before next retry attempt will be executed.
    pub after: Option<(Duration, Option<Instant>)>,
}

impl From<RetryOptionsWithDeadline> for RetryOptions {
    fn from(v: RetryOptionsWithDeadline) -> Self {
        Self { retries: v.retries, after: v.after.map(|(at, _)| at) }
    }
}

impl RetryOptionsWithDeadline {
    /// Returns [`Duration`] after which a [`Scenario`] could be retried. If
    /// [`None`], then [`Scenario`] is ready for the retry.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(super) fn left_until_retry(&self) -> Option<Duration> {
        let (dur, instant) = self.after?;
        dur.checked_sub(instant?.elapsed())
    }
}

/// Alias for [`fn`] used to determine whether a [`Scenario`] is [`Concurrent`]
/// or a [`Serial`] one.
///
/// [`Concurrent`]: ScenarioType::Concurrent
/// [`Serial`]: ScenarioType::Serial
/// [`Scenario`]: gherkin::Scenario
pub type WhichScenarioFn = fn(
    &gherkin::Feature,
    Option<&gherkin::Rule>,
    &gherkin::Scenario,
) -> ScenarioType;

/// Alias for [`Arc`]ed [`Fn`] used to determine [`Scenario`]'s
/// [`RetryOptions`].
///
/// [`Scenario`]: gherkin::Scenario
pub type RetryOptionsFn = Arc<
    dyn Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
        &Cli,
    ) -> Option<RetryOptions>,
>;

/// Alias for [`fn`] executed on each [`Scenario`] before running all [`Step`]s.
///
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
pub type BeforeHookFn<World> = for<'a> fn(
    &'a gherkin::Feature,
    Option<&'a gherkin::Rule>,
    &'a gherkin::Scenario,
    &'a mut World,
) -> LocalBoxFuture<'a, ()>;

/// Alias for [`fn`] executed on each [`Scenario`] after running all [`Step`]s.
///
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
pub type AfterHookFn<World> = for<'a> fn(
    &'a gherkin::Feature,
    Option<&'a gherkin::Rule>,
    &'a gherkin::Scenario,
    &'a event::ScenarioFinished,
    Option<&'a mut World>,
) -> LocalBoxFuture<'a, ()>;

/// Alias for a failed [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
pub(super) type IsFailed = bool;

/// Alias for a retried [`Scenario`].
///
/// [`Scenario`]: gherkin::Scenario
pub(super) type IsRetried = bool;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_options_next_try() {
        let opts = RetryOptions {
            retries: Retries::initial(2),
            after: Some(Duration::from_secs(1)),
        };

        let next = opts.next_try().unwrap();
        assert_eq!(next.retries.left, 1);
        assert_eq!(next.after, Some(Duration::from_secs(1)));

        let next2 = next.next_try().unwrap();
        assert_eq!(next2.retries.left, 0);
        
        let last = next2.next_try();
        assert!(last.is_none());
    }

    #[test]
    fn test_scenario_type() {
        assert_eq!(ScenarioType::Serial, ScenarioType::Serial);
        assert_ne!(ScenarioType::Serial, ScenarioType::Concurrent);
    }

    #[test]
    fn test_retry_options_with_deadline() {
        let opts = RetryOptions {
            retries: Retries::initial(1),
            after: Some(Duration::from_millis(100)),
        };

        let now = Instant::now();
        let with_deadline = opts.with_deadline(now);
        
        assert_eq!(with_deadline.retries, opts.retries);
        assert!(with_deadline.after.is_some());
    }

    #[test]
    fn test_retry_options_without_deadline() {
        let opts = RetryOptions {
            retries: Retries::initial(1),
            after: Some(Duration::from_millis(100)),
        };

        let without_deadline = opts.without_deadline();
        
        assert_eq!(without_deadline.retries, opts.retries);
        assert!(without_deadline.after.is_some());
        assert!(without_deadline.after.unwrap().1.is_none());
    }
}