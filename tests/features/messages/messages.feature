@messages34 @phase4
Feature: Portable Cucumber execution reports
  Consumers can read bounded native Rust runs as standard Cucumber Messages
  without mistaking unsupported source or execution information for a valid run.

  Scenario Outline: Exchange source and execution outcomes with standard consumers
    Given a "<run>" plain English feature run
    When the run is reported as Cucumber Messages
    Then every envelope validates against the official Messages 34.2.1 schema
    And source, AST, pickle and execution references are complete
    And the reported step statuses are "<statuses>"
    And the completed run reports success "<success>"

    Examples:
      | run        | statuses                  | success |
      | passing    | PASSED,PASSED             | true    |
      | failing    | FAILED,SKIPPED            | false   |
      | undefined  | UNDEFINED,SKIPPED         | false   |
      | ambiguous  | AMBIGUOUS,SKIPPED         | false   |
      | background | PASSED,PASSED,PASSED,PASSED| true    |
      | parallel   | PASSED,PASSED,PASSED,PASSED| true    |
      | empty      |                           | true    |

  Scenario Outline: Preserve official pickle keyword context
    Given a "<run>" plain English feature run
    When the run is reported as Cucumber Messages
    Then every envelope validates against the official Messages 34.2.1 schema
    And source, AST, pickle and execution references are complete
    And the pickle step types are "<types>"

    Examples:
      | run                  | types                   |
      | initial conjunction  | Unknown,Unknown         |
      | wildcard             | Context,Unknown,Unknown |
      | inherited background | Action,Action           |

  Scenario Outline: Refuse unsupported information instead of fabricating interoperability
    Given a "<run>" unsupported feature run
    When the run is reported as Cucumber Messages
    Then reporting fails visibly with "<reason>"
    And no successful completed run is emitted

    Examples:
      | run          | reason                |
      | outline      | scenario outlines     |
      | table        | step arguments        |
      | docstring    | step arguments        |
      | rule         | rules                 |
      | tags         | tags                  |
      | description  | descriptions          |
      | language     | English               |
      | missing source | source unavailable  |
      | changed source | differs from runner AST |
      | empty with background | empty scenarios with backgrounds |
