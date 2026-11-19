@phase35
Feature: Inspect selected scenarios without invoking application callbacks
  Dry-run constructs selected Worlds and classifies definitions, preserving
  diagnostics without failing for undefined or ambiguous application steps.

  Scenario Outline: Classifications remain visible without executing definitions
    Given a dry-run application configured for "<mode>"
    When the selected application is inspected
    Then its step outcomes are "<statuses>"
    And inspection succeeds once without invoking callbacks or deadlines
    And schema-valid Messages preserve the classification and successful run in the official Query

    Examples:
      | mode      | statuses                              |
      | known     | SKIPPED                               |
      | undefined | UNDEFINED                             |
      | ambiguous | AMBIGUOUS                             |
      | mixed     | SKIPPED,UNDEFINED,SKIPPED,AMBIGUOUS     |

  Scenario: Retained scenario selections describe skipped decisions
    Given a dry-run application configured for "observed hooks"
    When the selected application is inspected
    Then selected scenario callbacks have dry-run skip decisions and excluded callbacks are absent
    And Messages contain skipped callback slots and the real native World initialization
    And inspection succeeds once without invoking callbacks or deadlines

  Scenario: Unobserved and replaced legacy hooks remain suppressed
    Given a dry-run application configured for "legacy hooks"
    When the selected application is inspected
    Then native setup and cleanup retain skipped metadata without invocation records
    And inspection succeeds once without invoking callbacks or deadlines

  Scenario: Global hooks retain honest skipped lifecycle decisions
    Given a dry-run application configured for "global hooks"
    When the selected application is inspected
    Then global selections are skipped in setup and reverse cleanup order without actual invocation records
    And Messages contain zero-duration skipped global decisions with real definition identities
    And inspection succeeds once without invoking callbacks or deadlines

  Scenario: An empty selected scenario still constructs its World
    Given a dry-run application configured for "empty"
    When the selected application is inspected
    Then one World is constructed and the empty attempt is skipped
    And inspection succeeds once without invoking callbacks or deadlines

  Scenario: Constructor context cannot acquire callback capabilities during inspection
    Given a dry-run application configured for "constructor context"
    When the selected application is inspected
    Then the constructor has no active attachment or completion scope
    And inspection succeeds once without invoking callbacks or deadlines

  Scenario: Actual constructor failure is not hidden by classification success
    Given a dry-run application configured for "constructor failure"
    When the selected application is inspected
    Then the actual initialization failure is reported without retry or callback invocation

  Scenario: Selection happens before World construction
    Given a dry-run application configured for "selection"
    When the selected application is inspected
    Then only the selected scenario constructs a World
    And inspection succeeds once without invoking callbacks or deadlines

  Scenario: Explicit execution mode is retained across runner builders and clones
    Given a dry-run application configured for "builder preservation"
    When the selected application is inspected
    Then its step outcomes are "SKIPPED"
    And inspection succeeds once without invoking callbacks or deadlines

  Scenario: Turning inspection off restores the retained outcome policy
    Given a dry-run application configured for "restore run"
    When the selected application is inspected
    Then ordinary invocation resumes and its configured pending outcome remains authoritative

  Scenario: Existing applications execute by default
    Given a dry-run application configured for "default run"
    When the selected application is inspected
    Then the ordinary body executes and legacy outcome metadata stays absent
