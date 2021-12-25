Feature: Animal feature

  Scenario: If we forgot to feed a hungry dog it will be hungry
    Given a hungry dog
    When I feed the dog
    Then the dog is not hungry

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry
