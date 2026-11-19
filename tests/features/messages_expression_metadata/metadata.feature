@phase32a_expression_metadata
Feature: Built-in expression provenance and argument groups survive Messages reporting
  Scenario Outline: Consumers receive actual original expressions and faithful built-in groups
    Given a metadata run using the "<fixture>" built-in expression
    When the run completes and the pinned JavaScript expression library reads its matches
    Then its original expression and full raw argument groups agree with the official library
    And only the actual used built-in parameter descriptors are emitted
    And the argument envelopes satisfy both supported Messages schemas
    Examples:
      | fixture              |
      | integer              |
      | float                |
      | word                 |
      | anonymous            |
      | string-double        |
      | string-single        |
      | string-empty         |
      | unicode-prefix       |
      | optional-alternative |
      | numeric-alias        |

  Scenario: Repeated string captures retain accurate spans despite the pinned index polyfill defect
    Given a metadata run using the "string-escaped" built-in expression
    When the run completes and the pinned JavaScript expression library reads its matches
    Then native JavaScript indices verify the truthful range and isolate the pinned nested-start discrepancy
    And only the actual used built-in parameter descriptors are emitted
    And the argument envelopes satisfy both supported Messages schemas

  Scenario: Ambiguous matches retain argument-list alignment with definition identities
    Given a metadata run with two matching built-in expressions
    When the run completes and the pinned JavaScript expression library reads its matches
    Then each reported argument list belongs to the definition at the same index
    And the argument envelopes satisfy both supported Messages schemas

  Scenario: Replacing a registration clears its obsolete expression metadata
    Given an expression registration replaced through the existing raw insertion API
    When the run completes and the pinned JavaScript expression library reads its matches
    Then only the raw regex remains and unavailable capture metadata is omitted
    And the argument envelopes satisfy both supported Messages schemas

  Scenario Outline: Unsupported metadata remains absent without changing existing execution
    Given a metadata run using "<registration>" registration
    When the run completes and the pinned JavaScript expression library reads its matches
    Then only the raw regex remains and unavailable capture metadata is omitted
    And the argument envelopes satisfy both supported Messages schemas
    Examples:
      | registration             |
      | legacy expression        |
      | unannotated nested regex |
      | external regex flags     |
      | undescribed custom type  |

  Scenario: Inconsistent explicit metadata fails visibly
    Given a raw registration carrying metadata that describes a different expression
    When that inconsistent registration executes
    Then reporting identifies the metadata mismatch without a successful run-finished envelope

  Scenario: Existing manual StepConstructor implementations remain source-compatible
    Given a manual StepConstructor implementing only the original required method
    When its collection executes with Messages reporting
    Then its step still passes with raw definition metadata
