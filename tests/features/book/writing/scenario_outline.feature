Feature: Animal feature

  Scenario Outline: If we feed a hungry animal it will no longer be hungry
    Given a hungry <animal>
    When I feed the <animal> <n> times
    Then the <animal> is not hungry

    Examples:
      | animal | n |
      | cat    | 2 |
      | dog    | 3 |
      | 🦀     | 4 |
