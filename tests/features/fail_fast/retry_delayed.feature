Feature: A
  @retry(2).after(2s)
  Scenario: 1
    Then step panics

  Scenario: 2
    Then nothing happens
