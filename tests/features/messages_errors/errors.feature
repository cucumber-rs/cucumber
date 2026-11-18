@phase17
Feature: Report input failures without inventing successful execution
  A failed source or outline expansion produces an actionable standard report
  with the original captured source when available and an unsuccessful run.

  Scenario Outline: Preserve failed source diagnostics after its file changes
    Given an input with a "<failure>" failure
    When parsing finishes and the source file is "<change>" before reporting
    Then the report contains the exact captured source and native diagnostic location
    And every error-report envelope satisfies the pinned Messages schema
    And the run completes unsuccessfully without invented test cases

    Examples:
      | failure   | change    |
      | syntax    | deleted   |
      | syntax    | rewritten |
      | expansion | deleted   |

  Scenario: A missing input is a run failure without an invented source
    Given an input with a "missing file" failure
    When the missing input is reported
    Then the report identifies the I/O failure without a fabricated parse source
    And every error-report envelope satisfies the pinned Messages schema
    And the run completes unsuccessfully without invented test cases

  Scenario: Valid scenarios still report alongside a rejected source
    Given a directory with valid and invalid feature inputs
    When the directory is run without fail-fast
    Then the passing scenario and rejected source are both represented
    And every error-report envelope satisfies the pinned Messages schema
    And the overall run is unsuccessful

  Scenario: A failed later read does not reuse an older source generation
    Given a successful source snapshot retained before its file is deleted
    When the missing input is reported
    Then the current input is cleared and the retained snapshot stays unchanged
    And the report identifies the I/O failure without a fabricated parse source
    And the run completes unsuccessfully without invented test cases
