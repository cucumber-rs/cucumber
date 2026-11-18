@phase37a @ordered_scheduling
Feature: Schedule a complete selected application in a reproducible global order
  Explicit scheduling orders actual case admission across all selected sources.
  Each logical case keeps its capacity through retries, and serial cases form
  exclusive barriers. Existing callers retain the legacy scheduler by default.

  Scenario Outline: Defined and reverse order cover features, Rules and Examples
    Given a selected application spanning two features, Rules and outline rows
    When the complete application runs in "<order>" order with one active case
    Then actual case starts follow the "<order>" global source sequence
    And every selected case executes once with its original source identity

    Examples:
      | order   |
      | defined |
      | reverse |

  Scenario Outline: Seeded random order matches the pinned JavaScript permutation
    Given a selected application spanning two features, Rules and outline rows
    When the complete application runs in random order with string seed "<seed>"
    Then actual case starts match the pinned JavaScript shuffle for that seed
    And repeating the run produces the same permutation

    Examples:
      | seed          |
      | 0             |
      | reproducible  |
      | café-🦀       |

  Scenario: Filtering precedes the global permutation
    Given an application with selected and excluded cases in several sources
    When the selected application runs in random order with string seed "selected"
    Then its actual starts match shuffling only the selected cases
    And excluded cases construct no World and invoke no callbacks

  Scenario: A delayed source producer can finish before any application callbacks
    Given a source stream whose second feature and completion wait on controlled gates
    When the application requests explicit reverse scheduling
    Then the producer reaches both gates without executor starvation
    And no World or application hook starts before source completion
    When the remaining source and completion gates are released
    Then actual case starts follow the complete reverse sequence

  Scenario: Source errors survive buffered plan collection
    Given a source stream containing valid cases and a parse error
    When the complete application runs in defined order without fail fast
    Then the parse error remains visible and makes the run unsuccessful
    And all valid selected cases execute in defined order

  Scenario: Concurrent cases overlap without exceeding configured capacity
    Given three ordered concurrent cases coordinated by entry and release gates
    When the complete application runs in defined order with two active cases
    Then the first two cases enter before either is released
    And the third case enters only after capacity is released
    And observed active case count never exceeds two

  Scenario: A serial case is an exclusive barrier in the selected global order
    Given the ordered cases concurrent A, concurrent B, serial C and concurrent D
    When the complete application runs in defined order with two active cases
    Then A and B may overlap but C waits until both have finished
    And D waits until C has finished
    And C never overlaps another case

  Scenario: A retry retains its logical case slot and fresh attempt identity
    Given a first case that fails once and succeeds on its second attempt
    And a following case waiting for the only execution slot
    When the complete application runs in defined order with one active case
    Then both attempts of the first case finish before the following case starts
    And retry attempts have distinct ScenarioIds and Worlds with unchanged source identity
    And terminal counters report two passed cases and one recovered failed attempt

  Scenario: A delayed retry retains its slot while unrelated capacity remains useful
    Given a retrying case with a controlled retry delay and two following cases
    When the complete application runs in defined order with two active cases
    Then the retrying case retains one slot while another case can progress
    And the delayed retry starts only after its delay gate is released
    And no more than two logical cases are active

  Scenario: A serial case retains exclusivity across retries and retry delay
    Given a serial case that retries after a controlled delay before another case
    When the complete application runs in defined order with two active cases
    Then the following case cannot enter during either serial attempt or its delay
    And it enters after the serial case reaches its terminal result

  Scenario: Fail fast observes terminal failure after retry exhaustion
    Given a first case that fails every permitted attempt and a following case
    When the complete application runs in defined order with one active case and fail fast
    Then all permitted attempts of the first case run
    And the following case never constructs a World or invokes callbacks
    And the run reports the terminal failure without duplicate feature finishes

  Scenario: JavaScript parallel zero explicitly selects serial execution
    Given three selected cases and explicit JavaScript parallel value zero
    When the complete application runs in defined order
    Then all three cases execute with an active case limit of one

  Scenario Outline: Native zero concurrency fails configuration instead of stalling
    Given native zero concurrency supplied through "<entry>"
    When the application resolves its execution configuration
    Then configuration rejects zero with a positive-concurrency diagnostic
    And no World or application hook is invoked

    Examples:
      | entry   |
      | CLI     |
      | builder |

  Scenario: Empty buffered input completes without application cases
    Given a source stream that completes without selected cases
    When the complete application runs in defined order
    Then the run completes without constructing a World or dispatching a case

  Scenario: Legacy callers keep their existing scheduling policy
    Given the existing legacy serial-priority fixture with positive concurrency
    When the application runs without explicit scheduling options
    Then the established legacy fixture dispatch and retry behavior is preserved
