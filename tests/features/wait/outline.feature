Feature: Outline

  @tag @fail_after
  Scenario Outline: wait
    Given <wait> secs
    When <wait> secs
    Then <wait> secs
      """
      Doc String
      """

    Examples:
      | wait |
      | 2    |
      | 1    |
      | 1    |

    @long
    Examples:
      | wait |
      | 5    |
