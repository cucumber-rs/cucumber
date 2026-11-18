@phase6 @messages_source
Feature: Source-faithful portable Gherkin reports
  Captured feature source remains the authority for AST and pickle identities,
  including outlines and rich arguments, when native execution filters scenarios.

  Scenario Outline: Match the official Gherkin 43 source graph
    Given the official Gherkin 43 fixture "<fixture>"
    When its captured source is converted to Messages
    Then its AST and pickles equal the official fixture modulo IDs and URI
    And every graph envelope validates against Messages 34.2.1

    Examples:
      | fixture                           |
      | rule_with_tag                     |
      | complex_background                |
      | descriptions                      |
      | several_examples                  |
      | escaped_pipes                     |
      | docstrings                        |
      | step_with_datatable_and_docstring  |
      | example_tokens_everywhere         |

  Scenario: Selected outline rows retain original source and example identities
    Given a tagged outline inside a rule with comments between example rows
    When only the selected expanded scenario runs with captured-source reporting
    Then the report contains the complete original unexpanded AST
    And the executed test case references the selected real example row
    And both argument values are normalized and ordered in its pickle
    And every graph envelope validates against Messages 34.2.1

  Scenario: Captured source survives deletion after parsing
    Given a feature captured before its file is deleted
    When its recorded native feature runs with captured-source reporting
    Then reporting succeeds using the captured bytes without a final newline
    And every graph envelope validates against Messages 34.2.1

  Scenario: Missing captured source fails visibly
    Given an executed feature absent from the shared source store
    When its recorded native feature runs with captured-source reporting
    Then reporting rejects the missing snapshot without a successful completion

  Scenario: Literal language directives in content retain their source meaning
    Given an English feature with a language directive inside a doc string and a later comment
    When its captured source is converted to Messages
    Then the document remains English and retains both literal directive lines
    And every graph envelope validates against Messages 34.2.1
