Feature: Outline

  @original@merged.tag # comment
  Scenario Outline: foo
    Given foo is <bar1>
    When foo is <bar2>
    Then foo is <bar3>

    # comment
    @examples_ # comment
    #comment
    Examples:
      | bar1 | bar2 | bar3 |
      |  0   |  1   |  2   |
