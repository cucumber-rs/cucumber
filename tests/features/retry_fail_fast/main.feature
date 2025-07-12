Feature: retry fail fast

  @serial
  Scenario Outline: attempts
    Given attempt <attempt>

    Examples:
      | attempt |
      | 1       |
      | 2       |
      | 3       |
      | 4       |
      | 5       |
      | 6       |
      | 7       |
