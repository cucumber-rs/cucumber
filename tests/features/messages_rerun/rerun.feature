@phase44
Feature: Derive an actionable rerun selection without losing original execution evidence
  Applications inspect a source-faithful Messages run, retain every attempt,
  and rerun only the selected original scenario or Examples row locations.

  Scenario Outline: Rerun membership follows protocol status rather than run success
    Given a completed protocol case with status "<status>" and run success "<success>"
    When the application collects its rerun plan
    Then the rerun selection contains "<membership>"
    And the formatted result agrees with the pinned JavaScript rerun formatter
    Examples:
      | status    | success | membership |
      | PASSED    | true    | none       |
      | SKIPPED   | true    | case       |
      | PENDING   | true    | case       |
      | UNDEFINED | false   | case       |
      | AMBIGUOUS | false   | case       |
      | FAILED    | false   | case       |
      | UNKNOWN   | true    | case       |

  Scenario: Recovered retries retain history but only exhausted cases are selected
    Given a real application with recovering and exhausted cases
    When the application collects its rerun plan
    Then every original retry attempt remains inspectable
    And only the exhausted terminal attempt contributes a rerun location
    And the formatted result agrees with the pinned JavaScript rerun formatter

  Scenario: A second native execution selects the exact original failed Examples rows
    Given a real filtered concurrent Rule outline with duplicate row values
    When the application collects its rerun plan
    And the application executes the safe local rerun selection
    Then only the original failed Examples row locations execute again
    And the original attempt denominator and evidence remain unchanged
    And the formatted result agrees with the pinned JavaScript rerun formatter

  Scenario: Grouped output preserves encounter order and duplicate source lines
    Given completed cases sharing source locations across multiple URIs
    When the application collects its rerun plan
    Then URI groups and repeated lines preserve the official encounter order
    And a configured separator agrees with the pinned JavaScript formatter

  Scenario: An interrupted started attempt remains UNKNOWN at the run boundary
    Given a run that finishes after a case starts without its finish event
    When the application collects its rerun plan
    Then the unfinished attempt is retained as UNKNOWN and selected
    And the formatted result agrees with the pinned JavaScript rerun formatter

  Scenario: An unfinished stream cannot claim a final rerun plan
    Given a protocol stream without testRunFinished
    When the application collects its rerun plan
    Then collection reports an incomplete run rather than a successful empty plan

  Scenario Outline: Missing source or execution references remain explicit errors
    Given a protocol stream with a missing "<reference>" reference
    When the application collects its rerun plan
    Then collection reports the missing "<reference>" reference without guessed locations
    Examples:
      | reference |
      | testCase  |
      | pickle    |
      | document  |
      | astNode   |

  Scenario: Run-level failure does not invent a scenario rerun location
    Given a failed protocol run with parse and global-hook failures but no case attempts
    When the application collects its rerun plan
    Then run failure remains visible and no scenario selection is invented
    And the formatted result agrees with the pinned JavaScript rerun formatter

  Scenario: A passing run produces an explicit empty local rerun selection
    Given a real application whose selected cases all pass
    When the application collects its rerun plan
    Then local selection is None and the URI resolver is never called
    And no unrestricted LineSelectors are used to launch a second run

  Scenario: URI resolution failures do not silently become local paths
    Given a completed protocol case with status "FAILED" and run success "false"
    When the application collects its rerun plan
    Then a rejecting URI resolver returns its error without a fallback path
