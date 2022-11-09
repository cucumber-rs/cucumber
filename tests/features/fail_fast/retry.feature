Feature: A
  @retry(2)
  Scenario: 1
    Then step panics

  Scenario: 2
    Then nothing happens
