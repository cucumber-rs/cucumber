@phase38b
Feature: Plain Gherkin table cell locations match official UTF16 column counting
  Scenario Outline: Astral content changes following cell columns without changing values
    Given a plain step table containing "<case>"
    When the source is compiled and executed with captured Messages
    Then the full AST and Pickles equal the pinned official Gherkin43 compiler
    And the real native handler receives every unchanged table cell
    And source bytes and row locations remain unchanged
    Examples:
      | case                         |
      | emoji before a following cell |
      | multiple astral cells        |
      | escaped pipe beside emoji    |
      | escaped backslash before emoji |
      | unknown escape before emoji  |
      | combining characters beside emoji |
      | NBSP cell padding and emoji  |
      | EM SPACE preserved inside a cell |

  Scenario: Examples header and body cells retain official columns
    Given an outline with astral Examples headers and values
    When the source is compiled and executed with captured Messages
    Then the full AST and Pickles equal the pinned official Gherkin43 compiler
    And real expanded handlers receive the official names and cell values
    And selecting an actual Examples row still references that original row

  Scenario: Unicode token indentation remains scalar while cell content uses UTF16
    Given a table with NBSP indentation and astral content before its second cell
    When the source is compiled and executed with captured Messages
    Then the full AST and Pickles equal the pinned official Gherkin43 compiler
    And source bytes and row locations remain unchanged

  Scenario: Markdown cell locations retain their existing typed representation
    Given a Markdown table with astral content before its second cell
    When the source is compiled and executed with captured Messages
    Then the full AST and Pickles equal the pinned official Gherkin43 compiler
    And the real native handler receives every unchanged table cell
