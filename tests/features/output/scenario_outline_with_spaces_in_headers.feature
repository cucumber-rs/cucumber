Feature: Outline

  Scenario Outline: foo
    Given foo is <bar 1>
    When foo is <bar two>
    Then foo is <bar three>

    Examples:
      | bar 1 | bar two | bar three |
      | 0     | 1       | 2         |
