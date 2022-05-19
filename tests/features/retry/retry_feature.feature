@retry(1)
Feature: Retry all scenarios under a feature tagged with @retry

  @retry-test-implicit-1
  Scenario: Retry this failing scenario one time
    Given a failing step

  @retry(2) @retry-override-implicit-2
  Scenario: Retry this failing scenario two times
    Given a failing step
