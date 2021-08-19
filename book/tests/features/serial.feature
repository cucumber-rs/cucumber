Feature: Animal feature

  @serial
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given A hungry cat
    When I feed the cat
    Then The cat is not hungry

  @serial
  Scenario: If we feed a satiated cat it will not become hungry
    Given A satiated cat
    When I feed the cat
    Then The cat is not hungry
