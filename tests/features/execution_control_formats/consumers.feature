@phase30b_formats
Feature: Built-in consumers preserve explicit execution outcomes
  Pending and skipped remain non-passing while strictness controls final failure.

  Scenario Outline: Built-in consumers agree on controlled outcomes
    Given an application reporting pending and skipped through "<consumer>"
    When its real execution finishes under strict and non-strict policies
    Then the consumer preserves non-passing status and correct final failure semantics

    Examples:
      | consumer       |
      | Basic          |
      | Summarize      |
      | FailOnSkipped  |
      | Repeat         |
      | Normalize Tee  |
      | Messages Query |
      | JSON           |
      | JUnit          |
      | libtest        |

  Scenario Outline: A recovered retry does not leave a false final failure
    Given an application whose pending attempt retries after cleanup failure through "<consumer>"
    When its real execution finishes under strict and non-strict policies
    Then the consumer reports the successful final attempt without stale pending failure counts

    Examples:
      | consumer       |
      | Basic          |
      | Summarize      |
      | FailOnSkipped  |
      | Repeat         |
      | Normalize Tee  |
      | Messages Query |
      | JSON           |
      | JUnit          |
      | libtest        |
