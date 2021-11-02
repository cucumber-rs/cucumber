Feature: Example feature

  Scenario Outline: An example scenario
    Given foo is 0
      | sample       | val1   | val2   |
      | longer value | <val1> | <val2> |
    When foo is 0

    Examples:
      | val1 | val2 |
      | 1    | 4    |
      | 2    | 5    |
      | 3    | 6    |

  Scenario: An example sync scenario
    Given foo is sync 0

  Scenario: Steps that return results
    When I write "abc" to "myfile"
    Then the file "myfile" should contain "abc"

  Scenario: Steps that return results and fail
    When I write "abc" to "myfile"
    Then the file "not-here" should contain "abc"
