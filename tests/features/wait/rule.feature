Feature: Basic
  Background:
    Given 1 sec

  @serial
  Scenario: 1 sec
    Given 1 sec
    When 1 sec
    Then unknown
    Then 1 sec

  Rule: rule
    @fail_after
    Scenario: 2 secs
      Given 2 secs
      When 2 secs
      Then 2 secs
      Then 1 sec
