@phase6_sources
Feature: Reports retain the exact source that was parsed
  A report must describe the executed input even if files later change or disappear.

  Scenario Outline: Source remains available after the filesystem changes
    Given a source document without a final newline
    When the parser captures the source and the file is <change>
    Then the snapshot contains the exact original bytes
    And the captured native feature matches the parsed feature
    Examples:
      | change    |
      | rewritten |
      | deleted   |

  Scenario: Outlines retain their original source and unexpanded AST
    Given a source document containing an outline
    When the parser captures the source and expands its examples
    Then the snapshot still contains the outline template
    And the runner input contains both expanded examples

  Scenario: The configured default dialect accompanies the source
    Given a French source document without a language directive
    When the French parser captures the source
    Then the snapshot records the French default dialect
    And the captured native feature matches the parsed feature

  Scenario: Captured source generations remain immutable when a store is reused
    Given a source document without a final newline
    When the same path is parsed again with different content
    Then the retained first snapshot contains the original bytes
    And the current snapshot contains the newly parsed bytes
