Feature: Rule Outline

  Rule: To them all
    Scenario Outline: wait
      Given <wait> secs
      When <wait> secs
      Then <wait> secs

      Examples:
        | wait |
        | 2    |
        | 1    |
        | 1    |
        | 5    |
