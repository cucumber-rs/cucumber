Feature: Animal feature

  Rule: Hungry cat becomes satiated

    Scenario: If we feed a hungry cat it will no longer be hungry
      Given a hungry cat
      When I feed the cat
      Then the cat is not hungry

  Rule: Satiated cat remains the same

    Scenario: If we feed a satiated cat it will not become hungry
      Given a satiated cat
      When I feed the cat
      Then the cat is not hungry
