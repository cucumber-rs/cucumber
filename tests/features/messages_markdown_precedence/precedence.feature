@phase45
Feature: Markdown literal content wins over tag syntax in executable tokens
  Scenario Outline: Tag-looking text remains literal in its executable context
    Given the Markdown precedence case "<case>"
    When its captured Markdown document executes against the pinned compiler
    Then its complete AST and Pickles match official Gherkin43
    And actual handlers receive the official literal text and arguments
    And its original source bytes and typed locations are retained
    Examples:
      | case                                |
      | first step inline tag               |
      | following step inline tag           |
      | feature background inline tag       |
      | rule background inline tag          |
      | data table inline tag               |
      | following data row inline tag       |
      | Examples header inline tag          |
      | Examples body inline tag            |
      | opening fence media inline tag      |
      | table then fence inline tag         |
      | fence then table inline tag         |
      | docstring content inline tag        |
      | standalone inherited tags           |
      | tag-looking declaration stays tags  |
      | consumed fence slot remains tags    |
      | consumed table slot remains tags    |

  Scenario Outline: Literal tag handling cannot legalize misplaced or repeated arguments
    Given the Markdown precedence case "<case>"
    When its captured Markdown document executes against the pinned compiler
    Then native and official parsing both reject the document
    And no application handler runs or successful completion is reported
    Examples:
      | case                                |
      | pending tag before literal step     |
      | duplicate literal docstrings        |
      | literal table docstring table       |
      | literal inconsistent rows           |
      | dangling standalone tag             |
