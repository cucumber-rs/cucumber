@phase14
Feature: Select official outline interpolation without changing legacy consumers
  Applications can preserve undefined placeholders and interpolate sequentially
  in header order, with identical values in handlers and portable Pickles.

  Scenario Outline: Execute and report the official substituted values
    Given an outline exercising "<case>" interpolation
    When it runs with Gherkin 43 example semantics
    Then handler names, steps, tables and docstrings match the official compiler
    And the emitted Pickle payloads match the official compiler
    And the snapshot keeps raw templates and the selected example policy

    Examples:
      | case              |
      | unknown token     |
      | forward cascade   |
      | reverse cascade   |
      | dollar values     |
      | spaced header     |
      | duplicate headers |
      | literal regexp    |

  Scenario: Default consumers retain errors for unknown placeholders
    Given an outline exercising "unknown token" interpolation
    When the default parser expands it
    Then the unknown placeholder remains a legacy expansion error

  Scenario: Default consumers retain single-pass substitution
    Given an outline exercising "forward cascade" interpolation
    When the default parser expands it
    Then its substituted values retain the inserted later placeholder
