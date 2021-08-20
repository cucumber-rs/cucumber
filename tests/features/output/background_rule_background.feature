Feature: output

  Background:
    Given foo is 0

  Rule: output

    Background:
      Given foo is 1

    Scenario: output
      Given foo is 2
      When foo is 3
      Then foo is 4
