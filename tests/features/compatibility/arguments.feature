@phase1
Feature: Portable feature steps carry a document and table together
  Both arguments reach the Rust step body regardless of their order in the feature.

  Scenario: A document precedes a table
    Given keyword independent matching
    And a feature containing:
      ```
      Feature: Payments
        Scenario: Payment
          When the payment is submitted
            """
            invoice 42
            """
            | amount | 42 |
      ```
    When the feature is executed
    Then 1 steps pass, 0 steps fail and 0 steps are skipped
    And the observed payment is "invoice 42" with amount "42"

  Scenario: A table precedes a document
    Given keyword independent matching
    And a feature containing:
      ```
      Feature: Payments
        Scenario: Payment
          When the payment is submitted
            | amount | 42 |
            """
            invoice 42
            """
      ```
    When the feature is executed
    Then 1 steps pass, 0 steps fail and 0 steps are skipped
    And the observed payment is "invoice 42" with amount "42"

  Scenario: Outline values expand in both arguments
    Given keyword independent matching
    And a feature containing:
      ```
      Feature: Payments
        Scenario Outline: Payment
          When the payment is submitted
            | amount | <amount> |
            """
            invoice <amount>
            """
          Examples:
            | amount |
            | 42     |
      ```
    When the feature is executed
    Then 1 steps pass, 0 steps fail and 0 steps are skipped
    And the observed payment is "invoice 42" with amount "42"

  Scenario: A second document is rejected
    Given keyword independent matching
    And a feature containing:
      ```
      Feature: Invalid payments
        Scenario: Payment
          When the payment is submitted
            """
            first
            """
            """
            second
            """
      ```
    When the feature is executed
    Then the feature is rejected before steps execute

  Scenario: A table document table sequence is rejected
    Given keyword independent matching
    And a feature containing:
      ```
      Feature: Invalid payments
        Scenario: Payment
          When the payment is submitted
            | amount | 42 |
            """
            invoice 42
            """
            | amount | 43 |
      ```
    When the feature is executed
    Then the feature is rejected before steps execute
