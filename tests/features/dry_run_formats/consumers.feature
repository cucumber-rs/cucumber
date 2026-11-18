@phase35_formats
Feature: Report inspection classifications without inventing execution failure
  Dry-run keeps undefined and ambiguous definitions visible while the selected
  policy controls final success. Genuine initialization failures remain failures.

  Scenario Outline: Inspection retains classifications without failing the report
    Given a "<consumer>" report for recognized, undefined and ambiguous definitions
    When the application is inspected without executing callbacks
    Then all classifications remain distinguishable and the report succeeds
    And three Worlds are constructed once with no callbacks or retries

    Examples:
      | consumer |
      | JSON     |
      | JUnit    |
      | libtest  |

  Scenario Outline: A real constructor failure still fails inspection
    Given a "<consumer>" report whose World constructor fails
    When the application is inspected without executing callbacks
    Then the report preserves the initialization failure and fails

    Examples:
      | consumer |
      | JSON     |
      | JUnit    |
      | libtest  |

  Scenario Outline: Returning to execution restores classification failure semantics
    Given a "<consumer>" report for recognized, undefined and ambiguous definitions
    When inspection is turned off and the application executes
    Then the recognized body passes while undefined and ambiguous cases fail the report

    Examples:
      | consumer |
      | JSON     |
      | JUnit    |
      | libtest  |
