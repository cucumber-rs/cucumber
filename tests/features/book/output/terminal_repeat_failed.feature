Feature: Animal feature

  Scenario: If we forgot to feed a hungry Felix it will be hungry
    Given a hungry cat
    Then the cat is not hungry

  Scenario: If we feed a hungry Felix it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry
