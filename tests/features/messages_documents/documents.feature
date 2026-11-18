@phase25
Feature: Explicit latest mode processes complete Gherkin documents
  Applications can report valid featureless source without inventing a Feature,
  scenario or execution, while existing Feature-only defaults remain unchanged.

  Scenario Outline: Valid featureless sources produce exact empty document reports
    Given a "<shape>" featureless source
    When it runs through the explicit latest Cucumber convenience and Messages
    Then its exact source and empty graph equal the pinned official parser
    And it has zero Pickles or test cases and a successful schema-valid run
    Examples:
      | shape                    |
      | blank                    |
      | whitespace               |
      | comments without newline |
      | CRLF comments            |

  Scenario: Featureless documents coexist with ordinary executed features
    Given a directory containing blank commented and ordinary feature files
    When it runs through the explicit latest Cucumber convenience and Messages
    Then every source has exactly one document and only the ordinary feature executes
    And all document envelopes satisfy the pinned schema

  Scenario Outline: Invalid documents cannot become successful empty reports
    Given a "<shape>" invalid source
    When it runs through the explicit latest Cucumber convenience and Messages
    Then the pinned official parser and native runtime both reject the document
    And it has no empty document record and no successful run
    Examples:
      | shape              |
      | arbitrary text     |
      | language directive |

  Scenario: Existing default parsing still requires a Feature
    Given a "blank" featureless source
    When it runs with default parser policies and source-backed Messages
    Then the legacy report contains a parse error and no empty document

  Scenario: Empty-document snapshots preserve the selected fallback dialect
    Given a "comments without newline" featureless source
    When a latest parser with French fallback captures the document
    Then its immutable empty record preserves exact bytes path and French fallback
    And Feature snapshots stay absent while input bytes remain available

  Scenario: Latest convenience validates the currently configured dialect
    Given a Basic parser configured with the legacy formal dialect
    When latest document semantics are requested
    Then the unsupported fallback is reported without silent replacement

  Scenario: Retained empty generations survive replacement invalid input and failed reads
    Given a "comments without newline" featureless source
    When successive captures replace it with a comment feature invalid text and missing file
    Then each current generation has the correct record type and retained empty bytes stay unchanged
