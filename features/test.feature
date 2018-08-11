Feature: Basic functionality

  Scenario: foo
    Given a thing
    When nothing

  Scenario: bar
    Given a thing
    When something goes wrong

  Rule: A rule
    
    Scenario: a scenario inside a rule
      Given I am in inside a rule
      Then things are working
      