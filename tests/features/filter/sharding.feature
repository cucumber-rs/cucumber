Feature: sharding

  @keep
  Scenario: feature scenario
    Given 1 < 10

  @drop
  Scenario: filtered feature scenario
    Given 2 < 10

  @keep
  Scenario Outline: outline scenario <number>
    Given <number> < 10

    Examples:
      | number |
      | 3      |
      | 4      |

  Rule: scenarios in a rule

    @keep
    Scenario: first rule scenario
      Given 5 < 10

    @keep
    Scenario: second rule scenario
      Given 6 < 10
