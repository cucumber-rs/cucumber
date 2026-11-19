@phase32b_custom_metadata
Feature: Explicit custom parameter metadata describes the actual registered Rust types
  Scenario Outline: Complete custom descriptors preserve actual source and raw argument trees
    Given a custom expression fixture "<fixture>" with explicitly declared snippet and preference flags
    When its native handler and source-backed Messages reporter execute
    Then the original custom expression and full raw argument groups agree with the pinned official expression library
    And the emitted custom descriptor retains the actual name and regex with the declared flags
    And existing Rust handler conversion remains unchanged
    And all emitted envelopes satisfy both supported Messages schemas
    Examples:
      | fixture             |
      | simple color        |
      | nested custom value |
      | absent custom group |
      | Unicode prefix      |

  Scenario: Existing custom types without metadata remain compatible
    Given a custom parameter implementing only the existing name and regex contract
    When its native handler and source-backed Messages reporter execute
    Then its handler still passes and unavailable expression metadata remains omitted
    And all emitted envelopes satisfy both supported Messages schemas

  Scenario: Previously available complete manual descriptors continue working
    Given a custom expression registered through the existing explicit metadata options
    When its native handler and source-backed Messages reporter execute
    Then its complete custom descriptor and argument groups remain available
    And all emitted envelopes satisfy both supported Messages schemas

  Scenario: Conflicting custom names fail visibly instead of overwriting provenance
    Given two complete custom descriptors with the same name and different actual regexes
    When their source-backed Messages reporter starts
    Then reporting identifies the descriptor conflict without a successful run-finished envelope

  Scenario: A descriptor cannot misidentify its actual custom Rust type
    Given an explicit custom descriptor whose name or regex differs from its registered type
    When its native handler and source-backed Messages reporter execute
    Then reporting rejects the inconsistent descriptor without fabricating metadata
