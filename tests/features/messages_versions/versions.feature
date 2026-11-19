@phase22
Feature: Select a standard Messages protocol explicitly
  Existing consumers retain Messages 34.2.1, while Gherkin 43 tooling can
  request Messages 35.0.0 without changing the observed execution or source.

  Scenario Outline: Emit supported source and execution in the chosen protocol
    Given a source-backed reporter configured for "<version>"
    When a rich outline with native hooks and retries executes
    Then the report declares "<version>" and satisfies its official schema
    And source, argument, hook and final attempt outcomes remain intact

    Examples:
      | version |
      | 34.2.1  |
      | 35.0.0  |

  Scenario: Existing constructors retain their protocol version
    Given a source-backed reporter using its default protocol
    When a rich outline with native hooks and retries executes
    Then the report declares "34.2.1" and satisfies its official schema
    And source, argument, hook and final attempt outcomes remain intact
