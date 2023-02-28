Feature: Basic

  @serial
  Scenario: deny skipped
    Given step 1

  Scenario Outline: steps
    Given step <step>

    Examples:
      | step |
      | 2    |
      | 3    |
      | 4    |
      | 5    |
