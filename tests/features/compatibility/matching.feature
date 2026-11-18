@phase1
Feature: Reuse a step vocabulary across Gherkin keywords
  Teams sharing feature files can opt into Cucumber.js matching without changing legacy suites.

  Scenario Outline: A single definition works under every primary keyword
    Given keyword independent matching
    And a <registration> definition for an account balance
    And a feature containing:
      """
      Feature: Shared vocabulary
        Scenario: Balance
          <keyword> the account balance is 42
      """
    When the feature is executed
    Then 1 steps pass, 0 steps fail and 0 steps are skipped
    And the observed balances are "42"
    Examples:
      | registration | keyword |
      | Given        | Given   |
      | Given        | When    |
      | Given        | Then    |
      | When         | Given   |
      | When         | When    |
      | When         | Then    |
      | Then         | Given   |
      | Then         | When    |
      | Then         | Then    |

  Scenario: Legacy consumers keep keyword specific lookup by default
    Given default matching
    And a Given definition for an account balance
    And a feature containing:
      """
      Feature: Existing vocabulary
        Scenario: Balance
          When the account balance is 42
      """
    When the feature is executed
    Then 0 steps pass, 0 steps fail and 1 steps are skipped
    And the observed balances are ""

  Scenario: Legacy consumers can retain equal patterns in different keyword groups
    Given default matching
    And a Given definition for an account balance
    And a Then definition for an account balance
    And a feature containing:
      """
      Feature: Existing vocabulary
        Scenario: Balance
          Given the account balance is 42
          Then the account balance is 42
      """
    When the feature is executed
    Then 2 steps pass, 0 steps fail and 0 steps are skipped
    And the observed balances are "42,42"

  Scenario: Ambiguous cross keyword definitions fail without invoking either body
    Given keyword independent matching
    And a Given definition for an account balance
    And a Then definition for an account balance
    And a feature containing:
      """
      Feature: Ambiguous vocabulary
        Scenario: Balance
          When the account balance is 42
      """
    When the feature is executed
    Then 0 steps pass, 1 steps fail and 0 steps are skipped
    And the output reports an ambiguous match
    And the observed balances are ""

  Scenario: Undefined phrases remain skipped
    Given keyword independent matching
    And a Given definition for an account balance
    And a feature containing:
      """
      Feature: Unknown vocabulary
        Scenario: Balance
          Then no definition exists
      """
    When the feature is executed
    Then 0 steps pass, 0 steps fail and 1 steps are skipped

  Scenario: Inherited and wildcard keywords reuse the same definition
    Given keyword independent matching
    And a Then definition for an account balance
    And a feature containing:
      """
      Feature: Shared vocabulary
        Scenario: Balances
          Given the account balance is 42
          And the account balance is 43
          But the account balance is 44
          * the account balance is 45
      """
    When the feature is executed
    Then 4 steps pass, 0 steps fail and 0 steps are skipped
    And the observed balances are "42,43,44,45"

  Scenario: Localized keywords share the same matching policy
    Given keyword independent matching
    And a Given definition for an account balance
    And a feature containing:
      """
      # language: fr
      Fonctionnalité: Shared vocabulary
        Scénario: Balance
          Alors the account balance is 42
      """
    When the feature is executed
    Then 1 steps pass, 0 steps fail and 0 steps are skipped
    And the observed balances are "42"
