@phase33
Feature: Explicit composable selection preserves legacy CLI precedence
  Scenario Outline: Typed repeated filters compose with application selection
    Given the selection fixture with inherited tags and expanded names
    And explicit filter configuration "<configuration>"
    When the application selects scenarios with its caller predicate
    Then exactly "<selected>" execute through real handlers
    And the captured source retains every original scenario and example row
    Examples:
      | configuration                | selected             |
      | names OR tags AND predicate  | invoice green        |
      | empty options                | invoice green, health|
      | empty tag expression         | invoice green        |
      | missing repeated tag         |                      |
      | expanded name                | invoice green        |

  Scenario Outline: Existing CLI filters join the explicit selection policy
    Given the selection fixture with inherited tags and expanded names
    And explicit filter configuration "<configuration>"
    And CLI filter configuration "<cli>"
    When the application selects scenarios with its caller predicate
    Then exactly "<selected>" execute through real handlers
    Examples:
      | configuration | cli                 | selected              |
      | expanded name | name alternative    | invoice green, health |
      | empty options | name and tag        | invoice green         |
      | empty options | excluded name       |                       |

  Scenario Outline: Legacy CLI selection still overrides the caller predicate
    Given the selection fixture with inherited tags and expanded names
    And legacy selection with CLI "<cli>"
    When the application selects scenarios with its caller predicate
    Then exactly "<selected>" execute through real handlers
    Examples:
      | cli                 | selected      |
      | excluded name       | invoice red   |
      | excluded tag        | invoice red   |
      | none                | invoice green, health |

  Scenario: Explicit policy survives clone and type-changing builders
    Given the selection fixture with inherited tags and expanded names
    And explicit filter configuration "expanded name"
    When the executor is cloned and its parser runner writer CLI and hook adapters are replaced
    Then exactly "invoice green" execute through real handlers
