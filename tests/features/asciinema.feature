Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry

  @dog
  Scenario: If we feed a satiated dog it will not become hungry
    Given a satiated dog
    When I feed the dog
    Then the dog is not hungry
