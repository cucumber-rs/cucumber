Feature: Outline

  Scenario Outline: foo
    Given foo is <bar1><bar1>
    When foo is <bar2><bar1><bar2>
    Then foo is <bar3><bar2>

    Examples:
      | bar1 | bar2 | bar3 |
      |  0   |  1   |  2   |
