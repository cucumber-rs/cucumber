Feature: Variety of scenario outcomes get exposed for integration
  Scenario: A successful scenario
    When something
    Then it's okay

  Scenario: A failing scenario
    When another thing
    Then it's not okay

  Scenario: A scenario with an unimplemented step
    When not implemented
    Then it's okay

  Scenario: A timing out scenario
    When something
    Then it takes a long time
