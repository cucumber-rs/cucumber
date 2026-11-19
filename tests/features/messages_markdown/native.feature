@phase28a
Feature: Native Markdown parsing retains official source semantics
  Scenario Outline: Complete official Markdown fixture graphs are preserved
    Given the official Markdown fixture "<fixture>"
    When its native Markdown document is parsed and compiled
    Then its entire Markdown graph equals the pinned expectation
    And its native projection retains original source positions and arguments
    Examples:
      | fixture               |
      | minimal.feature.md    |
      | misc.feature.md       |
      | datatables.feature.md |
      | docstrings.feature.md |
      | tags.feature.md       |

  Scenario Outline: Matcher boundaries agree with the pinned official Markdown parser
    Given the Markdown boundary "<boundary>"
    When its native Markdown document is parsed and compiled
    Then its entire Markdown graph equals the live pinned oracle
    And its native projection retains original source positions and arguments
    Examples:
      | boundary                      |
      | French fallback               |
      | language-looking prose        |
      | bullets and Unicode columns   |
      | indentation and separator rows|
      | variable fences and media     |
      | table then docstring          |
      | docstring then table          |
      | rule backgrounds and examples |
      | blank first line              |
      | empty source                  |

  Scenario Outline: Invalid Markdown arguments fail visibly
    Given the Markdown boundary "<boundary>"
    When its native Markdown document is parsed and compiled
    Then both native Markdown and the pinned oracle reject it
    Examples:
      | boundary              |
      | duplicate docstrings  |
      | table docstring table |
      | inconsistent rows     |
      | unterminated fence    |
      | dangling tags         |

  Scenario: Markdown remains independently available from Messages output
    Given the official Markdown fixture "minimal.feature.md"
    When its native Markdown document is parsed and compiled
    Then the parser API returns an owned native feature without a reporting dependency
