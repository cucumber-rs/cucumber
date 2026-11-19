@phase21_tags
Feature: Use pinned official tag expressions without changing legacy CLI parsing
  Explicit tag expressions can select scenarios and hooks using inherited tags.

  Scenario Outline: Evaluate official tag syntax
    Given the tag compatibility case "<case>"
    When the explicit tag expression is parsed and evaluated
    Then validity and selection equal the pinned official tag implementation

    Examples:
      | case |
      | empty |
      | whitespace |
      | literal without at |
      | leading trailing whitespace |
      | precedence |
      | parentheses |
      | repeated not |
      | escaped parentheses |
      | escaped space |
      | escaped tab |
      | unicode whitespace |
      | not prefix literal |
      | trailing escape |
      | invalid escape |
      | missing operand |
      | missing operator |
      | unmatched parentheses |

  Scenario: Select an expanded example using inherited feature and rule tags
    Given tagged feature rule and example rows
    When an explicit official expression filters the real run
    Then only the selected example runs and matching scenario hooks execute

  Scenario: Legacy tag parsing remains available
    Given an empty official tag expression
    Then explicit official evaluation selects all tags
    And legacy TagOperation parsing still rejects the empty expression
