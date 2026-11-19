@phase5 @scheduling
Feature: Async scenario scheduling preserves application isolation
  Concurrent scenarios may share execution capacity while retaining their own
  World state. Serial scenarios require exclusive access to the runner.

  Scenario Outline: Valid limits bound active scenarios and permit useful overlap
    Given 6 scenarios with independent World state
    And a concurrency limit of "<limit>"
    When the coordinated feature is executed
    Then all 6 scenarios pass with isolated World values
    And the observed peak concurrency is <peak>
    And no scenarios remain active

    Examples:
      | limit     | peak |
      | 1         | 1    |
      | 2         | 2    |
      | 3         | 3    |
      | unlimited | 6    |

  Scenario Outline: Serial tags reserve exclusive execution across tag scopes
    Given concurrent scenarios and serial scenarios tagged at "<scope>" scope
    And a concurrency limit of "2"
    When the coordinated feature is executed
    Then all 6 scenarios pass with isolated World values
    And the observed peak concurrency is 2
    And both serial scenarios ran exclusively
    And no scenarios remain active

    Examples:
      | scope    |
      | scenario |
      | rule     |
      | feature  |

  Scenario: Fail fast leaves queued scenarios unstarted at concurrency one
    Given 3 scenarios whose first step fails
    And a concurrency limit of "1"
    When the coordinated feature is executed with fail fast
    Then exactly one scenario fails and the other scenarios never start
    And no scenarios remain active
