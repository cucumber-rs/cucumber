Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat 1 time
    Then the cat is not hungry

  Scenario: If we feed a satiated cat it will not become hungry
    Given a 'tiny tiny' cat
    Then the cat is "tiny tiny"
