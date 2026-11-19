@phase13
Feature: One official doc-string value across execution and portable reports
  The opt-in Gherkin 43 policy delivers the same content as the pinned official
  JavaScript parser, while existing applications retain legacy values.

  Scenario Outline: Native handlers and Messages agree with the official parser
    Given a "<delimiter>" doc-string using "<edge>" syntax and "<newline>" line endings
    When the feature runs with Gherkin 43 doc-string semantics and captured Messages
    Then the actual handler payload equals the official Pickle content
    And the complete source AST and Pickles equal the pinned official parser modulo IDs and URI
    And doc-string media type and delimiter location match the official AST
    And every envelope satisfies Messages 34 and the run passed
    Examples:
      | delimiter | edge                 | newline |
      | quotes    | closing suffix       | LF      |
      | backticks | closing suffix       | LF      |
      | quotes    | repeated escapes     | LF      |
      | backticks | repeated escapes     | LF      |
      | quotes    | NBSP indentation     | LF      |
      | backticks | NBSP indentation     | LF      |
      | quotes    | mixed Unicode indent | LF      |
      | backticks | mixed Unicode indent | CRLF    |
      | quotes    | BOM indentation      | LF      |
      | backticks | inactive escapes     | LF      |
      | quotes    | short indentation    | CRLF    |
      | quotes    | whitespace boundary  | LF      |
      | backticks | all whitespace       | LF      |

  Scenario: Legacy payload remains independent of normalized report content
    Given a legacy doc-string containing repeated escaped quotes
    When the feature runs with legacy doc-string semantics and captured Messages
    Then the handler retains the exact legacy payload
    And the complete source AST and Pickles equal the pinned official parser modulo IDs and URI
    And every envelope satisfies Messages 34 and the run passed

  Scenario: Legacy delimiter acceptance remains unchanged
    Given a doc-string delimiter indented with NBSP
    When it is parsed with the legacy native policy
    Then the legacy parser still rejects that newly opt-in syntax
