@phase12
Feature: Exchange native reports with official Cucumber.js consumers
  A downstream query library and formatter must resolve actual report graphs
  and final outcomes without Rust-specific report processing.

  Scenario: Render a tagged Rule outline with both step arguments
    Given a rich native execution report
    When the pinned official JavaScript consumers read the report
    Then the official query resolves both example rows and their arguments
    And the official JUnit formatter reports two passing examples

  Scenario: Report final outcomes after recovery and exhausted retries
    Given a mixed retry execution report
    When the pinned official JavaScript consumers read the report
    Then the official query retains two final outcomes from four attempts
    And the official JUnit formatter reports one passing and one failing case
