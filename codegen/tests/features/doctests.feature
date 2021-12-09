Feature: Doctests

  Scenario: Foo
    Given foo is 0

  @allow.skipped
  Scenario: Bar
    Given foo is not bar
