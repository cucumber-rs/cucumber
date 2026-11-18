@phase19
Feature: Adopt the complete official Gherkin 43 plaintext fixture corpus
  Every good fixture retains its complete AST and compiled Pickles.
  Every bad fixture is rejected without claiming diagnostic wording parity.

  Scenario Outline: Accepted source <fixture> retains its complete official graph
    Given the official good plaintext fixture "<fixture>"
    When it is processed with all explicit Gherkin 43 source policies
    Then the entire graph equals its official expectation modulo IDs and URI
    And every graph envelope validates against Messages 34
    Examples:
      | fixture |
      | background.feature |
      | complex_background.feature |
      | conjunctions.feature |
      | datatables.feature |
      | datatables_with_new_lines.feature |
      | descriptions.crlf.feature |
      | descriptions.feature |
      | descriptions_with_comments.feature |
      | docstrings.crlf.feature |
      | docstrings.feature |
      | empty.feature |
      | escaped_pipes.feature |
      | example_token_multiple.feature |
      | example_tokens_everywhere.feature |
      | extra_table_content.feature |
      | i18n_emoji.feature |
      | i18n_fr.feature |
      | i18n_no.feature |
      | incomplete_background_1.feature |
      | incomplete_background_2.feature |
      | incomplete_feature_1.feature |
      | incomplete_feature_2.feature |
      | incomplete_feature_3.feature |
      | incomplete_scenario.feature |
      | incomplete_scenario_outline.feature |
      | language.feature |
      | minimal-example.feature |
      | minimal.crlf.feature |
      | minimal.feature |
      | padded_example.feature |
      | prefixed-keywords.feature |
      | readme_example.feature |
      | rule.feature |
      | rule_with_tag.feature |
      | rule_without_name_and_description.feature |
      | scenario_outline.feature |
      | scenario_outline_no_newline.feature |
      | scenario_outline_with_docstring.feature |
      | scenario_outline_with_value_with_dollar_sign.feature |
      | scenario_outline_with_value_with_trailing_backslash.feature |
      | scenario_outlines_with_tags.feature |
      | several_examples.feature |
      | spaces_in_language.feature |
      | star-keywords.feature |
      | step_with_datatable_and_docstring.feature |
      | tagged_feature_with_scenario_outline.feature |
      | tags.feature |
      | trim_space.feature |
      | trim_tab.feature |
      | very_long.feature |

  Scenario Outline: Invalid source <fixture> fails native document parsing
    Given the official bad plaintext fixture "<fixture>"
    When it is processed with all explicit Gherkin 43 source policies
    Then native document parsing rejects that official bad fixture
    Examples:
      | fixture |
      | backslash_at_end_of_line_in_datatable.feature |
      | file_ends_with_open_docstring.feature |
      | inconsistent_cell_count.feature |
      | invalid_language.feature |
      | multiple_parser_errors.feature |
      | not_gherkin.feature |
      | repeated_step_docstring.feature |
      | single_parser_error.feature |
      | unexpected_end_of_file.feature |
      | unexpected_eof.feature |
      | unfinished_datatable.feature |
      | whitespace_in_tags.feature |

