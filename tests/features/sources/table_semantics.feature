@phase15_sources
Feature: Applications explicitly reject inconsistent table sizes
  Strict table sizing rejects malformed inputs before handlers run and snapshots
  record the selected policy. Existing applications retain legacy truncation.

  Scenario Outline: Strict applications reject shorter and longer table rows
    Given an application "<kind>" table with a "<width>" later row
    When the table application runs with "Gherkin 43" sizing
    Then one parse error prevents every table handler from running
    And the invalid table source has no successful snapshot

    Examples:
      | kind     | width   |
      | step     | shorter |
      | step     | longer  |
      | Examples | shorter |
      | Examples | longer  |

  Scenario Outline: Valid strict table values reach real handlers
    Given an application "<kind>" table with a "matching" later row
    When the table application runs with "Gherkin 43" sizing
    Then the "<kind>" table values reach the handler exactly
    And the source snapshot records "Gherkin 43" table sizing

    Examples:
      | kind     |
      | step     |
      | Examples |

  Scenario Outline: Existing applications preserve excess-cell truncation
    Given an application "<kind>" table with a "longer" later row
    When the table application runs with "default" sizing
    Then the "<kind>" table values reach the handler exactly
    And the source snapshot records "Legacy" table sizing

    Examples:
      | kind     |
      | step     |
      | Examples |

  Scenario Outline: Basic parser exposes a typed strict-table failure
    Given an application "<kind>" table with a "longer" later row
    When Basic parses the table with strict sizing
    Then Basic reports the inconsistent-cell-count diagnostic

    Examples:
      | kind     |
      | step     |
      | Examples |
