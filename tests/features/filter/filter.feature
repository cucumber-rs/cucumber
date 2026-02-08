Feature: filtering
  
  Scenario Outline: by examples
    Given <bar1> < 10
    When <bar2> < 10
    Then <bar3> < 10

    Examples:
      | bar1 | bar2 | bar3 |
      |  0   |  1   |  2   |
      |  10  |  11  |  12  |
      |  20  |  21  |  22  |
