Feature: Outline

  Scenario Outline: foo
    Given foo is <bar1>
    When foo is <bar2>
    Then foo is <bar3>

    Examples:
      | bar1 | bar2 | bar3 |
      | 0    |  1   |  2   |
