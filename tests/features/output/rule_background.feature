Feature: output

  Rule: output

    Background:
      Given foo is 0

    Scenario: output
      Given foo is 1
      When foo is 2
      Then foo is 3
