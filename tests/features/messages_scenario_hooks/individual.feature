@phase31
Feature: Navigate observed individual scenario callbacks through standard Messages
  A report consumer can identify each selected callback and its actual outcome,
  distinguish skipped selections, and retrieve evidence from its originating scope.

  Scenario Outline: Report ordered callback slots and actual outcomes
    Given an observed scenario registry configured for "<mode>"
    When the native attempt is reported with its scenario-hook records
    Then every selected callback has a stable TestStep and real source definition
    And executed callbacks preserve their actual timestamps and result
    And skipped callback slots do not claim execution or elapsed duration
    And the official Query resolves callback results and final success "<success>"

    Examples:
      | mode              | success |
      | passing           | true    |
      | first setup fails | false   |
      | cleanup fails     | false   |
      | both stages fail  | false   |
      | inherited tags    | true    |
      | empty native      | true    |
      | empty js setup    | false   |

  Scenario: Retry keeps selected callback identities and fresh invocation evidence
    Given an observed scenario registry configured for "recovering retry"
    When the native attempt is reported with its scenario-hook records
    Then one TestCase keeps stable callback slots across two attempts
    And each attempt reports its own callback outcomes and final retry decision

  Scenario: Attachments belong to their actual callback
    Given an observed scenario registry configured for "callback attachments"
    When the native attempt is reported with its scenario-hook records
    Then the official Query retrieves each payload from its originating callback
    And stale callback handles cannot attach to another callback

  Scenario: Concurrent outline cases retain their callback evidence
    Given an observed scenario registry configured for "concurrent outline"
    When the native attempt is reported with its scenario-hook records
    Then each expanded row retains its own callback invocations and evidence

  Scenario: Legacy stage replacement retains composite reporting where necessary
    Given an observed scenario registry configured for "legacy stage replacement"
    When the native attempt is reported with its scenario-hook records
    Then selected registry callbacks are individual and the replaced stage is composite

  Scenario Outline: Constructor evidence retains its observed native boundary
    Given an observed scenario registry configured for "<mode>"
    When the native attempt is reported with its scenario-hook records
    Then constructor evidence is not attributed to a callback that has not started
    And the official Query resolves callback results and final success "<success>"

    Examples:
      | mode                    | success |
      | constructor attachment  | true    |
      | constructor failure     | false   |
