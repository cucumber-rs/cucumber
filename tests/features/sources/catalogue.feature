@phase7_sources
Feature: Applications select an official dialect catalogue explicitly
  Existing keyword behavior remains available while projects adopt Gherkin 43 dialects.

  Scenario: The selected catalogue applies to a source language directive
    Given a Texan source using the corrected Rule keyword
    When the parser captures it with the Gherkin 43 catalogue
    Then the captured feature contains the Texan rule and its scenario
    And the snapshot records the Gherkin 43 catalogue

  Scenario: Existing parsers retain the legacy catalogue by default
    Given a source document without a final newline
    When the parser captures the source and the file is rewritten
    Then the snapshot records the legacy catalogue

  Scenario: Changing catalogue never silently replaces an unsupported configured dialect
    Given a parser configured for the legacy formal dialect
    When the parser selects the Gherkin 43 catalogue
    Then the catalogue change reports the unsupported formal dialect
