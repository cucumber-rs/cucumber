@phase2 @portability
Feature: A portable Rust BDD foundation includes its compatible parser
  A project adopting this Cucumber source must receive the supported Gherkin
  behavior without knowing the original author's checkout layout.

  Scenario: An unrelated application consumes a relocated checkout
    Given a relocated Cucumber source checkout with its vendored parser
    And an application depending only on that relocated Cucumber checkout
    When the application parses a step with a table followed by a doc string
    Then the application observes both step arguments unchanged
    And the application uses no sibling parser checkout or Cargo patch
