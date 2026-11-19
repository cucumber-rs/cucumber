@phase10
Feature: Report native retry attempts to portable consumers
  A report preserves each observed attempt and only treats its final outcome as
  the scenario result. Undefined steps follow the native runner retry policy.

  Scenario Outline: Report the actual retry lifecycle
    Given a retrying feature with outcome "<outcome>"
    When it runs with a retry budget of <budget> through the Messages writer
    Then the report contains <attempts> distinct attempts of one test case
    And attempt retry flags are "<flags>"
    And attempt first-step statuses are "<statuses>"
    And the final run success is "<success>"
    And all retry envelopes satisfy the pinned schema and reference graph

    Examples:
      | outcome   | budget | attempts | flags            | statuses             | success |
      | recovered | 2      | 2        | true,false       | FAILED,PASSED        | true    |
      | exhausted | 2      | 3        | true,true,false  | FAILED,FAILED,FAILED | false   |
      | passing   | 2      | 1        | false            | PASSED               | true    |
      | undefined | 2      | 1        | false            | UNDEFINED            | false   |
      | exhausted | 0      | 1        | false            | FAILED               | false   |

  Scenario: Keep retry outcomes separate across concurrent scenarios
    Given concurrent recovering and exhausted scenarios
    When they run through the Messages writer
    Then each scenario has its own contiguous attempt sequence
    And the recovered scenario does not hide the exhausted failure
    And all retry envelopes satisfy the pinned schema and reference graph
