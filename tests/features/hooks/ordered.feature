@phase3
Feature: Reusable ordered scenario lifecycle hooks
  Applications compose setup and teardown without losing earlier registrations.

  Scenario: Setup follows registration order and teardown unwinds in reverse order
    Given the lifecycle configuration "ordered"
    When the application feature runs
    Then the lifecycle trace is "before-one,before-two,background,step,after-two,after-one"
    And the run has 0 failed steps and 0 hook errors

  Scenario: A setup failure stops later setup and steps but still runs teardown
    Given the lifecycle configuration "before-failure"
    When the application feature runs
    Then the lifecycle trace is "before-one,after-two,after-one"
    And the run has 0 failed steps and 1 hook errors
    And the hook diagnostic contains "before-one"

  Scenario: Teardown continues after asynchronous failures and reports every failure
    Given the lifecycle configuration "after-failures"
    When the application feature runs
    Then the lifecycle trace is "before-one,before-two,background,step,after-two,after-one"
    And the run has 0 failed steps and 1 hook errors
    And the hook diagnostic contains "after-two"
    And the hook diagnostic contains "after-one"

  Scenario: Teardown continues when a callback panics before returning its future
    Given the lifecycle configuration "sync-after-failure"
    When the application feature runs
    Then the lifecycle trace is "before-one,before-two,background,step,after-sync,after-two,after-one"
    And the run has 0 failed steps and 1 hook errors
    And the hook diagnostic contains "after-sync"

  Scenario: Teardown sees the failed scenario and still releases resources
    Given the lifecycle configuration "step-failure"
    When the application feature runs
    Then the lifecycle trace is "before-one,before-two,background,step,after-two,after-one"
    And the run has 1 failed steps and 0 hook errors
    And teardown observes a failed step

  Scenario: Conditional hooks use inherited feature rule and scenario tags
    Given the lifecycle configuration "tagged"
    When the application feature runs
    Then the lifecycle trace is "tagged-before,background,step,tagged-after"
    And the run has 0 failed steps and 0 hook errors

  Scenario: Each retry gets fresh scenario state and repeats its hooks
    Given the lifecycle configuration "retry"
    When the application feature runs
    Then the lifecycle trace is "before-one,before-two,background,step,after-two,after-one,before-one,before-two,background,step,after-two,after-one"
    And both attempts start with fresh world state
    And the run has 0 failed steps and 0 hook errors

  Scenario: Legacy replacement hooks retain last-registration behavior
    Given the lifecycle configuration "legacy"
    When the application feature runs
    Then the lifecycle trace is "before-two,background,step,after-two"
    And the run has 0 failed steps and 0 hook errors

  Scenario: After-only hooks can observe a scenario with no initialized world
    Given the lifecycle configuration "after-only"
    When the application feature runs
    Then the lifecycle trace is "after-without-world"
    And the run has 0 failed steps and 0 hook errors
