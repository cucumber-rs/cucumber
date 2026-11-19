@phase30b
Feature: Applications explicitly mark work pending or skipped without panicking
  Native consumers can distinguish a deliberate completion decision from an
  undefined definition, preserve cleanup, and agree on the final run outcome.

  Scenario Outline: Pending follows explicit strictness without retrying
    Given execution control "<location> pending" with strict pending "<strict>"
    When the application runs with the explicit JS outcome policy and retries
    Then the report contains a PENDING outcome and no invented panic
    And pending completes one attempt with final success "<success>"
    And all applicable cleanup completes

    Examples:
      | location        | strict | success |
      | step            | true   | false   |
      | step            | false  | true    |
      | before scenario | true   | false   |
      | after scenario  | true   | false   |
      | before step     | true   | false   |
      | after step      | false  | true    |

  Scenario Outline: Deliberate skip succeeds and retains its real meaning
    Given execution control "<location> skipped" with strict pending "true"
    When the application runs with the explicit JS outcome policy and retries
    Then the report contains deliberate SKIPPED without UNDEFINED
    And deliberate skip completes one successful attempt
    And all applicable cleanup completes

    Examples:
      | location        |
      | step            |
      | before scenario |
      | after scenario  |
      | before step     |
      | after step      |

  Scenario: Step-hook control outcomes do not suppress the body unless setup failed
    Given execution control "step hooks control" with strict pending "true"
    When the application runs with the explicit JS outcome policy and retries
    Then every selected step hook and the resolved body execute
    And the actual aggregate uses the most severe outcome

  Scenario Outline: Definition lookup follows the pinned skip distinction
    Given execution control "<mode>" with strict pending "false"
    When the application runs with the explicit JS outcome policy and retries
    Then later step statuses are "<statuses>"

    Examples:
      | mode                          | statuses                     |
      | explicit skip then unresolved | SKIPPED,SKIPPED,SKIPPED       |
      | pending then unresolved       | PENDING,UNDEFINED,AMBIGUOUS   |
      | failure then unresolved       | FAILED,UNDEFINED,AMBIGUOUS    |

  Scenario: Cleanup failure overrides pending and enables a fresh retry
    Given execution control "pending then cleanup failure" with strict pending "true"
    When the application runs with the explicit JS outcome policy and retries
    Then only the actual failure enables a retry with a fresh World
    And the final pending attempt retains its strict failure outcome

  Scenario: Scoped handles cannot change another callback or attempt
    Given execution control "concurrent scoped handles" with strict pending "false"
    When the application runs with the explicit JS outcome policy and retries
    Then closed handles reject requests and concurrent cases retain their own outcome
    And a second request cannot replace the first completion decision

  Scenario: A pending request is not an implicit return and cannot hide a failure
    Given execution control "pending then panic or timeout" with strict pending "true"
    When the application runs with the explicit JS outcome policy and retries
    Then the actual panic or timeout remains FAILED
    And expiry closes the completion scope

  Scenario: Existing World constructors keep working through the additive context
    Given original and attachment-aware World implementations with no execution-control override
    When each application runs using its existing constructors
    Then their constructor behavior and attachment scope remain unchanged

  Scenario: Existing custom writer source remains compatible
    Given a custom writer using the existing native event variants
    When an explicitly controlled application runs through that writer
    Then the custom writer compiles and sees a documented non-passing projection
    And metadata-aware consumers retain the exact completion status

