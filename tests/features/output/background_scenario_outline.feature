Feature: Outline

  Background:
    Given foo is 0

  Scenario Outline: foo
    Given foo is <bar1>
    When foo is <bar2>
    Then foo is <bar 3>

    Examples:
      | bar1 | bar2 | bar 3 |
      | 1    |  2   |   3   |
