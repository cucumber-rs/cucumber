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
      | 5    |
      | 6    |
      | 7    |

  Scenario: too many
    Given step 8
    When step 8
    Then step 8
    Then step 8
