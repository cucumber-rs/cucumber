@phase30_timeouts
Feature: Applications opt into bounded callback execution
  Existing applications retain unlimited execution. Opt-in deadlines apply to
  each actual body or hook and never silently remove cleanup.

  Scenario: Existing applications retain unlimited execution by default
    Given an application using the legacy timeout defaults
    When its gated step is released after an injected deadline would have elapsed
    Then the step passes without a timeout

  Scenario: The explicit JavaScript preset chooses five seconds
    Given an application using the JavaScript timeout preset
    When it invokes a step with no definition override
    Then the callback receives a five-second deadline

  Scenario: A timed-out body is cancelled before cleanup
    Given an application whose body remains pending with scoped attachments
    When its callback deadline expires
    Then its body is dropped and reports a timeout failure
    And its retained attachment handle rejects a later write
    And cleanup still executes

  Scenario: A timed-out attempt retries with fresh state
    Given an application whose first body attempt exceeds its deadline
    When it runs with one retry and a completing second attempt
    Then both attempts retain distinct failure and success evidence

  Scenario Outline: Definition overrides remain local
    Given two application steps with the first override "<override>"
    When their inherited and overridden deadlines are exercised
    Then only the first step uses "<override>" and the second keeps the default

    Examples:
      | override |
      | longer   |
      | disabled |

  Scenario Outline: Hooks use their own deadlines
    Given an application whose "<stage>" callback remains pending
    When that callback deadline expires
    Then its timeout is reported at the actual hook or containing step
    And the remaining cleanup follows the established lifecycle policy

    Examples:
      | stage           |
      | before scenario |
      | after scenario  |
      | before step     |
      | after step      |
      | before run      |
      | after run       |

  Scenario: Concurrent executions do not share deadline state
    Given two overlapping scenarios with independent callback deadlines
    When only one deadline expires and the other body completes
    Then one scenario fails and the other passes without crossed attribution

  Scenario: Timeouts run without a Tokio runtime
    Given an application on a plain futures executor
    When a real timer expires while its body remains pending
    Then the timeout is reported without requiring a runtime-specific reactor
