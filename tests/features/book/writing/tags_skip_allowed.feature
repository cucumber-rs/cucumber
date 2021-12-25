Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry

  @allow.skipped
  Scenario: If we feed a satiated cat it will not become hungry
    Given a wild cat
    When I feed the cat
    Then the cat is not hungry
