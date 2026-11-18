@phase20
Feature: Ordered native step lifecycle callbacks
  Reusable step hooks apply to executable background and scenario steps,
  preserve the world, and record each callback actually invoked.

  Scenario Outline: Preserve ordered setup and reverse cleanup on every outcome
    Given a step lifecycle configured for "<case>"
    When the native scenario executes
    Then the callback and body trace is "<trace>"
    And the after-step callbacks observe "<body>"
    And the final native failed step count is <failures>
    And each actual hook invocation has its own identity, scope and outcome

    Examples:
      | case          | trace                                      | body    | failures |
      | passing       | before-one,before-two,body,after-two,after-one | passed  | 0 |
      | before fails  | before-one,before-two,after-two,after-one      | not run | 1 |
      | body fails    | before-one,before-two,body,after-two,after-one | failed  | 1 |
      | after fails   | before-one,before-two,body,after-two,after-one | passed  | 1 |
      | all fail      | before-one,before-two,after-two,after-one      | not run | 1 |

  Scenario: Undefined steps do not invoke step hooks
    Given a step lifecycle configured for "undefined"
    When the native scenario executes
    Then no step hook or body invocation occurs
    And the native scenario reports one skipped step

  Scenario: Ambiguous steps do not invoke step hooks
    Given a step lifecycle configured for "ambiguous"
    When the native scenario executes
    Then no step hook or body invocation occurs
    And the final native failed step count is 1

  Scenario: Background and scenario steps receive inherited tag-selected callbacks
    Given a tagged feature and Rule with a background and scenario step
    When the native scenario executes
    Then selected hooks surround both steps and excluded hooks never run
    And each actual hook invocation has its own identity, scope and outcome

  Scenario: Retried steps have separate attempt identities and fresh worlds
    Given a step lifecycle configured for "retry"
    When the native scenario executes
    Then both attempts have complete hook traces and distinct execution identities
    And only the first attempt has a failed hook record
    And each actual hook invocation has its own identity, scope and outcome

  Scenario: Later steps skipped after a failure have no hook invocations
    Given a step lifecycle configured for "later skipped"
    When the native scenario executes
    Then hook records refer only to the first executed step
    And the final native failed step count is 1
