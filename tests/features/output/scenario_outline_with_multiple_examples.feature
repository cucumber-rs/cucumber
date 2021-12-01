Feature: Outline

  @original
  Scenario Outline: foo
    Given foo is <bar1>
    When foo is <bar2>
    Then foo is <bar3>

    @examples
    Examples:
      | bar1 | bar2 | bar3 |
      |  0   |  1   |  2   |

    @other-examples
    Examples:
      | bar1 | bar2 | bar3 |
      |  3   |  4   |  5   |
