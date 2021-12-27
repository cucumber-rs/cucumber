Feature: Animal feature

  Scenario: If we feed a hungry animal it will no longer be hungry
    Given a hungry animal
      | animal |
      | cat    |
      | dog    |
      | 🦀     |
    When I feed the animal multiple times
      | animal | times |
      | cat    | 2     |
      | dog    | 3     |
      | 🦀     | 4     |
    Then the animal is not hungry
