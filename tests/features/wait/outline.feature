Feature: Outline

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
      | 5    |
