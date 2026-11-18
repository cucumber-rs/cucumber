@phase8 @localized_messages
Feature: Localized source graphs use the parser's selected vocabulary
  Portable reports retain each source dialect, exact keyword and Unicode
  location without interpreting localized text as English approximations.

  Scenario Outline: Match official localized Gherkin 43 source graphs
    Given the official localized Gherkin 43 fixture "<fixture>"
    When its captured source is converted to Messages
    Then its AST and pickles equal the official fixture modulo IDs and URI
    And every graph envelope validates against Messages 34.2.1

    Examples:
      | fixture    |
      | i18n_fr    |
      | i18n_emoji |

  Scenario: The captured fallback dialect applies without a language directive
    Given a legacy French feature using the parser fallback without a directive
    When its captured source is converted to Messages
    Then the graph language is "fr" with step keywords "Soit ,Quand ,Alors "
    And the pickle step types are localized as "Context,Action,Outcome"
    And every graph envelope validates against Messages 34.2.1

  Scenario: A newly registered dialect retains no-space step keywords
    Given a Gherkin 43 Malayalam feature using the parser fallback without a directive
    When its captured source is converted to Messages
    Then the graph language is "ml" with step keywords "നൽകിയത്,എപ്പോൾ,പിന്നെ"
    And the pickle step types are localized as "Context,Action,Outcome"
    And every graph envelope validates against Messages 34.2.1

  Scenario: Latest-only Rule keywords retain localized structure and Unicode columns
    Given a Gherkin 43 Chinese feature with a latest-only Rule keyword and Unicode table cells
    When its captured source is converted to Messages
    Then the graph language is "zh-CN" with a Rule keyword "规则"
    And its Unicode table cell columns are 9 and 14
    And every graph envelope validates against Messages 34.2.1

  Scenario: Multiply classified localized keywords remain Unknown
    Given a Gherkin 43 French feature with wildcard and conjunction steps
    When its captured source is converted to Messages
    Then the pickle step types are localized as "Context,Unknown,Unknown"
    And every graph envelope validates against Messages 34.2.1

  Scenario: Legacy vocabulary does not acquire a latest-only structural keyword
    Given a legacy Chinese feature containing the latest-only Rule spelling
    When its captured source is converted to Messages
    Then the legacy graph retains that spelling as feature description
    And every graph envelope validates against Messages 34.2.1
