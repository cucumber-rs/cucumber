@phase24
Feature: Complete native run setup and cleanup with honest failure outcomes
  Global setup and cleanup run once per native executor invocation, without a
  scenario World. Every registered callback is attempted even after a failure.

  Scenario Outline: Continue global hooks and gate scenarios on setup success
    Given a run lifecycle configured for "<case>"
    When the native run completes with global hooks
    Then the run lifecycle trace is "<trace>"
    And global hook statistics report <hook errors> failures
    And global records identify each actual callback and its result

    Examples:
      | case         | trace                                             | hook errors |
      | passing      | before-one,before-two,body,after-two,after-one      | 0 |
      | before fails | before-one,before-two,after-two,after-one           | 1 |
      | before both  | before-one,before-two,after-two,after-one           | 2 |
      | before construct fails | before-one,before-two,after-two,after-one | 1 |
      | after construct fails | before-one,before-two,body,after-two,after-one | 1 |
      | after fails  | before-one,before-two,body,after-two,after-one      | 1 |
      | all fail     | before-one,before-two,after-two,after-one           | 4 |
      | body fails   | before-one,before-two,body,after-two,after-one      | 0 |

  Scenario: Global callbacks run for an empty feature
    Given a run lifecycle configured for "empty"
    When the native run completes with global hooks
    Then the run lifecycle trace is "before-one,before-two,after-two,after-one"
    And global records identify each actual callback and its result

  Scenario: Retries do not repeat global callbacks
    Given a run lifecycle configured for "retry"
    When the native run completes with global hooks
    Then the run lifecycle trace is "before-one,before-two,body,body,after-two,after-one"
    And global hook statistics report 0 failures
    And global records identify each actual callback and its result

  Scenario: A setup failure produces an unsuccessful run_and_exit process
    Given a run lifecycle configured for "before fails"
    When an isolated process runs it using run_and_exit
    Then the process fails with a global hook diagnostic

  Scenario: A cleanup failure produces an unsuccessful run_and_exit process
    Given a run lifecycle configured for "after fails"
    When an isolated process runs it using run_and_exit
    Then the process fails with a global hook diagnostic

  Scenario: Concurrent scenarios share one global setup and cleanup
    Given a run lifecycle configured for "parallel"
    When the native run completes with global hooks
    Then the run lifecycle trace is "before-one,before-two,body,body,after-two,after-one"
    And global records identify each actual callback and its result

  Scenario: Sequential reuse does not change a completed run's failure count
    Given a reusable run-hook registry and observation store
    When one failing run and one passing run execute sequentially
    Then their completed statistics and callback records remain separate
