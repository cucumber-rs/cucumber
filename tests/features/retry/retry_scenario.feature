Feature: Retry the tagged scenarios

  @retry(0) @retry-explicit-0
  Scenario: Do not retry this failing scenario
    Given a failing step

  @retry(1) @retry-explicit-1
  Scenario: Retry this failing scenario one time
    Given a failing step

  @retry(3) @retry-explicit-3
  Scenario: Retry this failing scenario three times
    Given a failing step

  @no_retry
  Scenario: This failing scenario shouldn't be retried
    Given a failing step
