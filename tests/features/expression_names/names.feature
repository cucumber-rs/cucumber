@phase42_expression_names
Feature: Explicit JavaScript expression definitions reject names the pinned registry forbids
  Scenario Outline: Invalid custom names fail while registering an explicit JavaScript expression
    Given a Rust custom parameter named "<name>"
    When an actual expr_js step definition is compiled at the completed BDD boundary
    Then registration fails with a parameter-name diagnostic
    And the pinned JavaScript parameter registry rejects the same name
    Examples:
      | name         |
      | closing)     |
      | square[name] |
      | dotted.name  |
      | dollar$name  |
      | pipe\|name   |
      | maybe?       |
      | many*        |
      | plus+        |

  Scenario Outline: Valid custom names remain portable
    Given a Rust custom parameter named "<name>"
    When an actual expr_js step definition is compiled and executed at the completed BDD boundary
    Then the native handler receives the expected custom value
    And the pinned JavaScript parameter registry accepts the same name
    Examples:
      | name         |
      | customer-id  |
      | account name |
      | 項目         |

  Scenario: Legacy custom names retain their existing behavior
    Given a Rust custom parameter named "dotted.name"
    When its existing derive and legacy expr step definition compile and execute at the completed BDD boundary
    Then the native handler receives the expected custom value
