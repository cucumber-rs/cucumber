@phase32b_raw_metadata
Feature: Explicit raw capture mappings report actual groups without guessing handler arguments
  Scenario Outline: Declared argument mappings preserve real group structure and source ranges
    Given a raw regex fixture "<fixture>" with an explicit argument capture mapping
    When its native handler and source-backed Messages reporter execute
    Then reported raw arguments follow the declared capture order and actual nested group structure
    And native JavaScript indices verify their raw values and UTF-16 positions
    And unknown parameter names and parameter descriptors remain omitted
    And all emitted envelopes satisfy both supported Messages schemas
    Examples:
      | fixture                  |
      | independent captures     |
      | one outer argument       |
      | flat nested arguments    |
      | absent and empty capture |
      | Unicode prefix           |

  Scenario: Invalid capture mappings cannot invent arguments
    Given a raw regex with a declared argument index outside its actual captures
    When the explicit raw metadata is constructed
    Then construction rejects the invalid capture mapping visibly

  Scenario: Hidden matcher settings cannot silently contradict explicit metadata
    Given explicit raw metadata paired with a matcher whose external flags change the observed match
    When its native handler and source-backed Messages reporter execute
    Then reporting rejects the inconsistent descriptor without fabricating metadata

  Scenario: Existing insertion clears obsolete raw capture metadata
    Given a raw registration with explicit capture metadata replaced through the existing insertion API
    When its native handler and source-backed Messages reporter execute
    Then its handler still passes and unavailable expression metadata remains omitted
    And all emitted envelopes satisfy both supported Messages schemas
