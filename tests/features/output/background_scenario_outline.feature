Feature: Outline

  Background:
    Given foo is 0

  Scenario Outline: foo
    Given foo is <bar1>
    When foo is <bar2>
    Then foo is <bar three>

    Examples:
      | bar1 | bar2 | bar three |
      | 1    |  2   |  3        |
