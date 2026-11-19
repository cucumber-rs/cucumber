@phase38
Feature: Explicit Gherkin43 lexical semantics preserve source and official token behavior
  Scenario Outline: Unicode token padding is interpreted without rewriting the source
    Given a classic source padded with "<padding>" around declarations tags steps and Examples
    When it is parsed and executed with explicit Gherkin43 lexical semantics
    Then the complete AST and Pickles equal the pinned official Gherkin43 compiler
    And real handlers receive the official trimmed names arguments and inherited tags
    And captured source bytes and native byte spans still identify the original declarations
    And selecting the actual padded Examples body row executes only that row
    Examples:
      | padding           |
      | NBSP              |
      | BOM               |
      | EM SPACE          |
      | IDEOGRAPHIC SPACE |
      | vertical tab      |
      | form feed         |
      | carriage return   |
      | line separator    |
      | paragraph separator |

  Scenario Outline: Token content uses ECMAScript whitespace instead of Rust whitespace
    Given the lexical source case "<case>"
    When it is parsed and executed with explicit Gherkin43 lexical semantics
    Then the complete AST and Pickles equal the pinned official Gherkin43 compiler
    And real handlers receive the official trimmed names arguments and inherited tags
    Examples:
      | case                           |
      | BOM trims titles and step text |
      | NEL remains title and step text |
      | NEL remains inside a tag       |
      | BOM separates tags            |
      | Unicode-indented directive and comment |
      | Unicode whitespace before a trailing tag comment |

  Scenario Outline: Characters outside ECMAScript whitespace are not token padding
    Given the lexical source case "<case>"
    When native and official compilers interpret the source
    Then both reject it without a successful run or invented source nodes
    Examples:
      | case                  |
      | NEL before Feature    |
      | NBSP inside a tag     |

  Scenario Outline: Featureless documents use the same explicit whitespace policy
    Given a featureless source containing "<padding>"
    When native and official compilers interpret the source
    Then both produce an empty document with the exact captured source
    Examples:
      | padding           |
      | NBSP              |
      | BOM               |
      | EM SPACE          |
      | line separator    |

  Scenario: Legacy token padding remains unchanged
    Given a classic source padded with "NBSP" around declarations tags steps and Examples
    When it is parsed with the default legacy lexical policy
    Then legacy parsing still rejects the Unicode token indentation

  Scenario: Legacy trailing NEL trimming remains unchanged
    Given the lexical source case "NEL remains title and step text"
    When it is parsed with the default legacy lexical policy
    Then native legacy values retain their previous Rust trimming behavior

  Scenario: Captured lexical policy survives source replacement and deletion
    Given a classic source padded with "EM SPACE" around declarations tags steps and Examples
    When the immutable source snapshot is retained and the file is replaced then deleted
    Then retained source graphs and row origins still use that snapshot's lexical policy

  Scenario: Lexical semantics are independent of the dialect catalogue
    Given a classic source padded with "NBSP" around declarations tags steps and Examples
    When it runs with Gherkin43 lexical semantics and the Legacy dialect catalogue
    Then the complete AST and Pickles equal the pinned official Gherkin43 compiler
    And real handlers receive the official trimmed names arguments and inherited tags
    And the captured catalogue stays Legacy while lexical semantics are Gherkin43
