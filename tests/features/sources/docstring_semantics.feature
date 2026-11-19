@phase9_sources
Feature: Applications choose the doc-string values delivered to step handlers
  Reports retain the original source and selected content policy while handlers
  receive either the preserved legacy value or explicitly adopted official content.

  Scenario: Official content reaches a real step handler without media-type text
    Given an application feature with a typed doc-string payload
    When the application runs with official doc-string semantics
    Then the handler receives the normalized payload exactly
    And the snapshot records official doc-string semantics and the original source

  Scenario: Outline values expand inside normalized doc strings before execution
    Given an application outline with two doc-string payload examples
    When the application runs with official doc-string semantics
    Then both handlers receive their expanded normalized payloads exactly
    And the snapshot keeps the unexpanded normalized doc-string template
    And the snapshot records official doc-string semantics and the original source

  Scenario: Existing applications retain legacy content without opting in
    Given an application feature with a legacy doc-string payload
    When the application runs with its default parser
    Then the handler receives the legacy payload including its edge newlines
    And the snapshot records legacy doc-string semantics and the original source
