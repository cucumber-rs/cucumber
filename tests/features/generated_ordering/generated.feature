@phase41
Feature: Replay a generated random case order from its observed seed
  Applications can request a random order without inventing a seed, retrieve
  the realized decimal seed, and reproduce the selected plan across languages.

  Scenario: Generated ordering exposes one actual decimal seed
    Given an application with explicit generated random ordering
    When the complete selected plan is permuted and executed
    Then one ordering record contains a decimal seed between 0 and 999999
    And the record describes the actual selected plan and permutation
    And the generated seed is reported exactly once before the first case starts

  Scenario: The realized seed replays the pinned JavaScript permutation
    Given an application with explicit generated random ordering
    When the complete selected plan is permuted and executed
    And its observed seed is replayed as an explicit seed
    Then both executions match the pinned JavaScript shuffle oracle for that seed
    And every selected case executes exactly once in each run

  Scenario: Replay applies to the globally filtered expanded plan
    Given generated ordering over filtered features and expanded outline rows
    When the complete selected plan is permuted and executed
    And its observed seed is replayed as an explicit seed
    Then only selected expanded cases appear in the replayed permutation
    And both executions match the pinned JavaScript shuffle oracle for that selected plan

  Scenario: A configured report sink receives the actual seed
    Given generated ordering with an application-owned seed report sink
    When the complete selected plan is permuted and executed
    Then the sink receives exactly one report matching the stored realization
    And the default generated-seed stderr report is suppressed

  Scenario: Reusing cloned options realizes once for each actual run
    Given cloned generated-order options sharing an ordering store
    When two separate selected plans are executed with those options
    Then there is one distinct run record for each actual permutation
    And each run reports exactly its own recorded decimal seed
    And replay is verified without requiring the two random seeds to differ

  Scenario: Explicit empty-string seeds retain their existing meaning
    Given an application with an explicit empty-string random seed
    When the complete selected plan is permuted and executed
    Then its order matches the pinned JavaScript empty-seed shuffle
    And no generated-seed report is emitted

  Scenario Outline: Non-generated orders do not acquire generated seed behavior
    Given an application with explicit "<order>" ordering
    When the complete selected plan is permuted and executed
    Then the selected plan follows its existing "<order>" ordering contract
    And no generated-seed report is emitted
    Examples:
      | order   |
      | defined |
      | reverse |

  Scenario: Native scheduling does not realize an ordering seed
    Given an application without explicit scheduling options
    When its selected cases are executed
    Then the unused ordering store remains empty
    And no generated-seed report is emitted

  Scenario: An empty selected plan still realizes one reported seed
    Given generated ordering whose selection excludes every case
    When the complete selected plan is permuted and executed
    Then no application World or case callback is constructed or invoked
    And one generated ordering record describes zero selected cases
    And the empty plan reports its recorded seed exactly once
