Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry
