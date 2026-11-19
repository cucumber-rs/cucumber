@phase26
Feature: Exchange actual global hook execution through standard Messages
  Consumers can distinguish global setup and cleanup failures from scenario
  results and resolve every observed hook invocation without fabricated events.

  Scenario Outline: Report global lifecycle outcomes and scenario gating
    Given a run with global hooks configured for "<case>"
    When its shared global-hook records are reported as Messages
    Then every observed global invocation has a definition and paired lifecycle
    And global setup is reported before scenarios and cleanup after them
    And the report contains <cases> scenario attempts and success "<success>"
    And the official consumer resolves the global-hook outcomes

    Examples:
      | case             | cases | success |
      | passing          | 1     | true    |
      | setup failure    | 0     | false   |
      | cleanup failure  | 1     | false   |
      | both failures    | 0     | false   |
      | empty input      | 0     | true    |

  Scenario: Sequential store reuse does not leak earlier run hooks
    Given two sequential runs sharing a global-hook store
    When each run is reported with its own Messages writer
    Then each report contains only its own observed global-hook invocations
    And both reports have complete unsuccessful or successful final outcomes
