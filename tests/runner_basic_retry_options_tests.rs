//! Tests for Basic runner RetryOptions parsing functionality.

use cucumber::runner::basic::{Cli, RetryOptions};
use cucumber::event::{Retries};
use gherkin::GherkinEnv;
use humantime::parse_duration;
use std::time::Duration;

mod scenario_tags {
    use super::*;

    // language=Gherkin
    const FEATURE: &str = r"
Feature: only scenarios
  Scenario: no tags
    Given a step

  @retry
  Scenario: tag
    Given a step

  @retry(5)
  Scenario: tag with explicit value
    Given a step

  @retry.after(3s)
  Scenario: tag with explicit after
    Given a step

  @retry(5).after(15s)
  Scenario: tag with explicit value and after
    Given a step
";

    #[test]
    fn empty_cli() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: None,
            retry_after: None,
            retry_tag_filter: None,
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
            None,
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }

    #[test]
    fn cli_retries() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: Some(7),
            retry_after: None,
            retry_tag_filter: None,
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }

    #[test]
    fn cli_retry_after() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: Some(7),
            retry_after: Some(parse_duration("5s").unwrap()),
            retry_tag_filter: None,
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }

    #[test]
    fn cli_retry_filter() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: Some(7),
            retry_after: None,
            retry_tag_filter: Some("@retry".parse().unwrap()),
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
            None,
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }

    #[test]
    fn cli_retry_after_and_filter() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: Some(7),
            retry_after: Some(parse_duration("5s").unwrap()),
            retry_tag_filter: Some("@retry".parse().unwrap()),
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
            None,
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }
}

mod rule_tags {
    use super::*;

    // language=Gherkin
    const FEATURE: &str = r#"
Feature: only scenarios
  Rule: no tags
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step

  @retry(3).after(5s)
  Rule: retry tag
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step
"#;

    #[test]
    fn empty_cli() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: None,
            retry_after: None,
            retry_tag_filter: None,
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[0],
                &cli
            ),
            None,
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[0],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 3 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }

    #[test]
    fn cli_retry_after_and_filter() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: Some(7),
            retry_after: Some(parse_duration("5s").unwrap()),
            retry_tag_filter: Some("@retry".parse().unwrap()),
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[0],
                &cli
            ),
            None,
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[0],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 3 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }
}

mod feature_tags {
    use super::*;

    // language=Gherkin
    const FEATURE: &str = r"
@retry(8)
Feature: only scenarios
  Scenario: no tags
    Given a step

  @retry
  Scenario: tag
    Given a step

  @retry(5)
  Scenario: tag with explicit value
    Given a step

  @retry.after(3s)
  Scenario: tag with explicit after
    Given a step

  @retry(5).after(15s)
  Scenario: tag with explicit value and after
    Given a step

  Rule: no tags
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step

  @retry(3).after(5s)
  Rule: retry tag
    Scenario: no tags
      Given a step

    @retry
    Scenario: tag
      Given a step

    @retry(5)
    Scenario: tag with explicit value
      Given a step

    @retry.after(3s)
    Scenario: tag with explicit after
      Given a step

    @retry(5).after(15s)
    Scenario: tag with explicit value and after
      Given a step
";

    #[test]
    fn empty_cli() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: None,
            retry_after: None,
            retry_tag_filter: None,
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .unwrap_or_else(|e| panic!("failed to parse feature: {e}"));

        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 8 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[0],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 8 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[0],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 3 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: None,
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 1 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }

    #[test]
    fn cli_retry_after_and_filter() {
        let cli = Cli {
            concurrency: None,
            fail_fast: false,
            retry: Some(7),
            retry_after: Some(parse_duration("5s").unwrap()),
            retry_tag_filter: Some("@retry".parse().unwrap()),
        };
        let f = gherkin::Feature::parse(FEATURE, GherkinEnv::default())
            .expect("failed to parse feature");

        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[0], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 8 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[1], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[2], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[3], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(&f, None, &f.scenarios[4], &cli),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[0],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 8 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[0]),
                &f.rules[0].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[0],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 3 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[1],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[2],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(5)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[3],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 7 },
                after: Some(Duration::from_secs(3)),
            }),
        );
        assert_eq!(
            RetryOptions::parse_from_tags(
                &f,
                Some(&f.rules[1]),
                &f.rules[1].scenarios[4],
                &cli
            ),
            Some(RetryOptions {
                retries: Retries { current: 0, left: 5 },
                after: Some(Duration::from_secs(15)),
            }),
        );
    }
}