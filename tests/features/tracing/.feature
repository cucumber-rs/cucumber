Feature: Basic

  @serial
  Scenario: deny skipped
    Given step 1
    When step 1
    Then unknown
    Then step 1

  @allow.skipped @serial
  Scenario: allow skipped
    Given step 2
    When step 2
    Then unknown
    Then step 2

  Scenario Outline: steps
    Given step <step>
    When step <step>
    Then step <step>

    Examples:
      | step |
      | 3    |
      | 4    |

  Scenario: too many
    Given step 5
    When step 5
    Then step 5
    Then step 5
