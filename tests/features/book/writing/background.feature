Feature: Animal feature

  Background:
    Given a hungry cat

  Rule: Hungry cat becomes satiated

    Scenario: If we feed a hungry cat it will no longer be hungry
      When I feed the cat
      Then the cat is not hungry

  Rule: Satiated cat remains the same

    Background:
      When I feed the cat

    Scenario: If we feed a satiated cat it will not become hungry
      When I feed the cat
      Then the cat is not hungry
