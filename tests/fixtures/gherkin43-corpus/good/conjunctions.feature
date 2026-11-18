Feature: Conjunctions

  You can avoid repeating the same keyword by using a conjunction. The
  conjunction step will inherit the step type from the previous step.

  Scenario: A
    Given a
    And a1
    But not a2
    When b
    And b3
    But not b4
    Then c
    But not c1
    And not c2

  Scenario: Stars keywords have the unknown step type, so their conjunctions are unknown too
    * a
    And a1

  Scenario: This lasts until the next known keyword
    * a
    And a1
    When b
    And b1
