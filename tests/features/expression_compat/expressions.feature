@phase21_expression
Feature: Select Cucumber.js expression parameter behavior explicitly
  Rust handlers can use the pinned JavaScript expression vocabulary and quote
  transformations while existing expr definitions retain native behavior.

  Scenario Outline: Match and transform an expression through a real handler
    Given the expression compatibility case "<case>"
    When the step runs through an explicit JavaScript-compatible expression
    Then its matched outcome and handler value equal the pinned official expression implementation

    Examples:
      | case |
      | negative integer |
      | unsigned integer |
      | plus integer rejected |
      | unicode integer rejected |
      | decimal float |
      | uppercase exponent |
      | lowercase exponent rejected |
      | trailing dot rejected |
      | infinity rejected |
      | NaN rejected |
      | word |
      | NEL word |
      | NBSP word rejected |
      | quoted string |
      | escaped double quote |
      | escaped single quote |
      | empty quoted string |
      | unescaped line separator |
      | escaped line separator rejected |
      | optional singular |
      | optional plural |
      | alternative first |
      | alternative second |
      | escaped literal |
      | anonymous |
      | double alias |
      | decimal alias |
      | long alias |
      | large integer alias |
      | custom parameter |
      | shadowed function name |
      | legacy shadowed function name |

  Scenario: Legacy float expressions retain their broader native grammar
    Given the expression compatibility case "legacy infinity"
    When the step runs through a legacy expression
    Then the legacy handler receives infinity

  Scenario: Legacy strings retain escaped quote characters
    Given the expression compatibility case "legacy escaped string"
    When the step runs through a legacy expression
    Then the legacy handler retains the quote escape
