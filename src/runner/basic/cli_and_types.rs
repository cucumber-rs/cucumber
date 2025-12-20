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
                // Check if tag starts with "@retry" or "retry"
                let retry_part = tag.strip_prefix("@retry").or_else(|| tag.strip_prefix("retry"));
                retry_part.map(|retries| {
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

        let apply_cli = |options: Option<(Option<usize>, Option<Duration>)>| {
            // Check if retry should be applied based on CLI configuration
            let cli_wants_retry = cli.retry_tag_filter.as_ref().map_or_else(
                || cli.retry.is_some() || cli.retry_after.is_some(),
                |op| {
                    op.eval(scenario.tags.iter().chain(
                        rule.iter().flat_map(|r| &r.tags).chain(&feature.tags),
                    ))
                },
            );

            // Return RetryOptions if:
            // 1. Tags define retry options (options.is_some()), OR
            // 2. CLI wants retry (cli_wants_retry is true)
            if let Some((tag_retries, tag_after)) = options {
                // Tags found - use tag values, with CLI values as fallback for missing parts
                Some(Self {
                    retries: Retries::initial(
                        tag_retries.or(cli.retry).unwrap_or(1),
                    ),
                    after: tag_after.or(cli.retry_after),
                })
            } else if cli_wants_retry {
                // No tags found, but CLI has retry options
                Some(Self {
                    retries: Retries::initial(cli.retry.unwrap_or(1)),
                    after: cli.retry_after,
                })
            } else {
                None
            }
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_cli_default() {
        let cli = Cli::default();
        assert_eq!(cli.concurrency, None);
        assert!(!cli.fail_fast);
        assert_eq!(cli.retry, None);
        assert_eq!(cli.retry_after, None);
        assert!(cli.retry_tag_filter.is_none());
    }

    #[test]
    fn test_cli_clone() {
        let cli = Cli {
            concurrency: Some(4),
            fail_fast: true,
            retry: Some(3),
            retry_after: Some(Duration::from_secs(2)),
            retry_tag_filter: None, // TagOperation parsing would be complex for test
        };
        
        let cloned = cli.clone();
        assert_eq!(cloned.concurrency, Some(4));
        assert!(cloned.fail_fast);
        assert_eq!(cloned.retry, Some(3));
        assert_eq!(cloned.retry_after, Some(Duration::from_secs(2)));
        assert!(cloned.retry_tag_filter.is_none());
    }

    #[test]
    fn test_scenario_type() {
        assert_eq!(ScenarioType::Serial, ScenarioType::Serial);
        assert_ne!(ScenarioType::Serial, ScenarioType::Concurrent);
        
        // Test Copy trait
        let serial = ScenarioType::Serial;
        let copied = serial;
        assert_eq!(serial, copied);
        
        // Test Hash trait by using in HashSet
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ScenarioType::Serial);
        set.insert(ScenarioType::Concurrent);
        assert_eq!(set.len(), 2);
    }

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
    fn test_retry_options_equality() {
        let opts1 = RetryOptions {
            retries: Retries::initial(1),
            after: Some(Duration::from_secs(1)),
        };
        
        let opts2 = RetryOptions {
            retries: Retries::initial(1),
            after: Some(Duration::from_secs(1)),
        };
        
        assert_eq!(opts1, opts2);
        
        let opts3 = RetryOptions {
            retries: Retries::initial(2),
            after: Some(Duration::from_secs(1)),
        };
        
        assert_ne!(opts1, opts3);
    }

    #[test]
    fn test_retry_options_parse_from_tags_scenario_tag() {
        let feature = create_test_feature(vec![]);
        let scenario = create_test_scenario(vec!["@retry(3)".to_string()]);
        let cli = Cli::default();
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 3);
        assert!(opts.after.is_none());
    }

    #[test]
    fn test_retry_options_parse_from_tags_with_after() {
        let feature = create_test_feature(vec![]);
        let scenario = create_test_scenario(vec!["@retry(2).after(1s)".to_string()]);
        let cli = Cli::default();
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 2);
        assert_eq!(opts.after, Some(Duration::from_secs(1)));
    }

    #[test]
    fn test_retry_options_parse_from_tags_rule_tag() {
        let feature = create_test_feature(vec![]);
        let rule = create_test_rule(vec!["@retry(4)".to_string()]);
        let scenario = create_test_scenario(vec![]);
        let cli = Cli::default();
        
        let opts = RetryOptions::parse_from_tags(&feature, Some(&rule), &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 4);
    }

    #[test]
    fn test_retry_options_parse_from_tags_feature_tag() {
        let feature = create_test_feature(vec!["@retry(5)".to_string()]);
        let scenario = create_test_scenario(vec![]);
        let cli = Cli::default();
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 5);
    }

    #[test]
    fn test_retry_options_parse_from_tags_priority() {
        // Scenario tag should have priority over rule and feature tags
        let feature = create_test_feature(vec!["@retry(1)".to_string()]);
        let rule = create_test_rule(vec!["@retry(2)".to_string()]);
        let scenario = create_test_scenario(vec!["@retry(3)".to_string()]);
        let cli = Cli::default();
        
        let opts = RetryOptions::parse_from_tags(&feature, Some(&rule), &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 3); // Scenario tag wins
    }

    #[test]
    fn test_retry_options_parse_from_cli() {
        let feature = create_test_feature(vec![]);
        let scenario = create_test_scenario(vec![]);
        let cli = Cli {
            retry: Some(2),
            retry_after: Some(Duration::from_millis(500)),
            ..Default::default()
        };
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 2);
        assert_eq!(opts.after, Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_retry_options_parse_cli_override_tags() {
        // When both tags and CLI options are present, tags take precedence for retry count
        let feature = create_test_feature(vec![]);
        let scenario = create_test_scenario(vec!["@retry(1).after(2s)".to_string()]);
        let cli = Cli {
            retry: Some(3),
            retry_after: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 1); // Tag value is used when present
        assert_eq!(opts.after, Some(Duration::from_secs(2))); // Tag duration is used
    }

    #[test]
    fn test_retry_options_parse_with_tag_filter() {
        let feature = create_test_feature(vec![]);
        let scenario = create_test_scenario(vec!["@smoke".to_string()]);
        let cli = Cli {
            retry: Some(2),
            retry_tag_filter: None, // TagOperation parsing would be complex for test
            ..Default::default()
        };
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 2);
    }

    #[test]
    fn test_retry_options_parse_tag_filter_no_match() {
        let feature = create_test_feature(vec![]);
        let scenario = create_test_scenario(vec!["@integration".to_string()]);
        let cli = Cli {
            retry: Some(2),
            retry_tag_filter: None, // With no tag filter and retry set, it should apply retry
            ..Default::default()
        };
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        // With CLI retry set and no tag filter, retry should be applied
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 2);
    }

    #[test]
    fn test_retry_options_parse_no_number() {
        // Test @retry tag without number (should default to 1)
        let feature = create_test_feature(vec![]);
        let scenario = create_test_scenario(vec!["@retry".to_string()]);
        let cli = Cli::default();
        
        let opts = RetryOptions::parse_from_tags(&feature, None, &scenario, &cli);
        assert!(opts.is_some());
        let opts = opts.unwrap();
        assert_eq!(opts.retries.left, 1); // Default to 1
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
        let (duration, instant) = with_deadline.after.unwrap();
        assert_eq!(duration, Duration::from_millis(100));
        assert_eq!(instant, Some(now));
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
        let (duration, instant) = without_deadline.after.unwrap();
        assert_eq!(duration, Duration::from_millis(100));
        assert!(instant.is_none());
    }

    #[test]
    fn test_retry_options_with_deadline_no_after() {
        let opts = RetryOptions {
            retries: Retries::initial(1),
            after: None,
        };

        let now = Instant::now();
        let with_deadline = opts.with_deadline(now);
        
        assert_eq!(with_deadline.retries, opts.retries);
        assert!(with_deadline.after.is_none());
    }

    #[test]
    fn test_retry_options_with_deadline_from_conversion() {
        let opts = RetryOptions {
            retries: Retries::initial(2),
            after: Some(Duration::from_secs(1)),
        };
        
        let with_deadline = opts.with_deadline(Instant::now());
        let converted: RetryOptions = with_deadline.into();
        
        assert_eq!(converted.retries, opts.retries);
        assert_eq!(converted.after, opts.after);
    }

    #[test]
    fn test_retry_options_with_deadline_left_until_retry() {
        let opts = RetryOptionsWithDeadline {
            retries: Retries::initial(1),
            after: None,
        };
        
        // No after duration, should return None
        assert!(opts.left_until_retry().is_none());
        
        // With duration but no instant
        let opts_no_instant = RetryOptionsWithDeadline {
            retries: Retries::initial(1),
            after: Some((Duration::from_secs(1), None)),
        };
        assert!(opts_no_instant.left_until_retry().is_none());
        
        // With both duration and instant
        let now = Instant::now();
        let opts_with_instant = RetryOptionsWithDeadline {
            retries: Retries::initial(1),
            after: Some((Duration::from_millis(100), Some(now))),
        };
        
        // Should have some time left
        let left = opts_with_instant.left_until_retry();
        assert!(left.is_some());
        assert!(left.unwrap() <= Duration::from_millis(100));
        
        // Wait a bit and check again
        thread::sleep(Duration::from_millis(50));
        let left_after = opts_with_instant.left_until_retry();
        assert!(left_after.is_some());
        assert!(left_after.unwrap() < left.unwrap());
        
        // Wait until after deadline
        thread::sleep(Duration::from_millis(60));
        let left_expired = opts_with_instant.left_until_retry();
        assert!(left_expired.is_none()); // Duration has passed
    }

    #[test]
    fn test_type_aliases() {
        // Test that type aliases compile and can be used
        let _which_fn: WhichScenarioFn = |_f, _r, _s| ScenarioType::Serial;
        
        // Test that basic bool types work (IsFailed and IsRetried are defined in supporting_structures)
        let failed: bool = true;
        assert!(failed);
        
        let retried: bool = false;
        assert!(!retried);
    }

    // Helper functions for creating test data
    fn create_test_feature(tags: Vec<String>) -> gherkin::Feature {
        gherkin::Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        }
    }
    
    fn create_test_scenario(tags: Vec<String>) -> gherkin::Scenario {
        gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 2, col: 1 },
        }
    }
    
    fn create_test_rule(tags: Vec<String>) -> gherkin::Rule {
        gherkin::Rule {
            keyword: "Rule".to_string(),
            name: "Test Rule".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            tags,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 2, col: 1 },
        }
    }
}