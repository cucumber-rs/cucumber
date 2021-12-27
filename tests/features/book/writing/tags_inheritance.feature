@feature
Feature: Animal feature

  @scenario
  Scenario Outline: If we feed a hungry animal it will no longer be hungry
    Given a hungry <animal>
    When I feed the <animal> <n> times
    Then the <animal> is not hungry

  @home
  Examples:
    | animal | n |
    | cat    | 2 |
    | dog    | 3 |

  @dire
  Examples:
    | animal | n |
    | lion   | 1 |
    | wolf   | 1 |
