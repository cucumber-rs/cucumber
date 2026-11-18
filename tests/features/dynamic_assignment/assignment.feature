@phase37b
Feature: Assign logical cases using the resources held by active cases
  Applications can keep shared browser and workspace resources exclusive while
  allowing independent work to overlap. Assignment observes actual case sources
  and retained retry reservations, without pretending to create worker isolation.

  Scenario: Cases sharing a browser resource cannot overlap
    Given ordered cases configured for "shared browser"
    When the application runs with explicit dynamic assignment
    Then cases sharing a browser resource execute exclusively
    And observed decisions refer to actual logical cases rather than worker identities

  Scenario: Independent browser and workspace resources can overlap
    Given ordered cases configured for "independent resources"
    When the application runs with explicit dynamic assignment
    Then independent cases overlap within the configured capacity
    And assignment callbacks see the previously admitted logical reservations

  Scenario: The first eligible queued case can bypass an ineligible concurrent case
    Given ordered cases configured for "first eligible"
    When the application runs with explicit dynamic assignment
    Then the first eligible candidate is admitted in queue order
    And an earlier rejected case remains queued for later consideration

  Scenario: No eligible candidate waits for actual progress
    Given ordered cases configured for "wait for release"
    When the application runs with explicit dynamic assignment
    Then blocked cases remain unassigned until the active case releases its reservation
    And the dispatcher waits without repeated idle decisions or unbounded record growth

  Scenario: A delayed retry retains its resource reservation
    Given ordered cases configured for "retry delay"
    When the application runs with explicit dynamic assignment
    Then the retrying logical case remains assigned throughout its delay
    And its fresh retry World does not create a second logical admission
    And independent resources can proceed while conflicting resources remain blocked

  Scenario: Assignment cannot bypass a queued Serial barrier
    Given ordered cases configured for "queued Serial barrier"
    When the application runs with explicit dynamic assignment
    Then no candidate after the first Serial barrier is evaluated or admitted early
    And the Serial case starts only after all preceding assigned cases complete

  Scenario: An active Serial case remains exclusive even if the predicate accepts everything
    Given ordered cases configured for "active Serial"
    When the application runs with explicit dynamic assignment
    Then the Serial case never overlaps another logical reservation

  Scenario: Idle fallback does not bypass a rejected Serial front
    Given ordered cases configured for "rejected Serial"
    When the application runs with explicit dynamic assignment
    Then idle fallback chooses the Serial front and preserves its exclusivity

  Scenario: An always-rejecting policy progresses with a visible aggregate warning
    Given ordered cases configured for "idle fallback"
    When the application runs with explicit dynamic assignment
    Then each idle fallback admits the queue head without claiming predicate eligibility
    And a single aggregate warning reports the actual idle intervention count on stderr
    And skipped predicate choices are not fabricated callback or worker execution records

  Scenario: An application receives fallback warnings through its configured sink
    Given ordered cases configured for "custom warning sink"
    When the application runs with explicit dynamic assignment
    Then the configured sink receives one typed aggregate warning
    And the same warning is retained in the assignment decision store
    And the default stderr warning is suppressed

  Scenario: Expanded outline cases retain their actual source and tag context
    Given ordered cases configured for "outline resource tags"
    When the application runs with explicit dynamic assignment
    Then the predicate observes expanded native source identities and inherited resource tags
    And decision snapshots preserve their fixed ordered-plan positions

  Scenario: Configuration survives cloning and type-changing builders
    Given ordered cases configured for "builder preservation"
    When the application runs with explicit dynamic assignment
    Then the installed policy and shared decision store remain active

  Scenario: Native scheduling remains unchanged without explicit assignment
    Given ordered cases configured for "default native"
    When the application runs without dynamic assignment
    Then ordinary native scheduling executes without assignment callbacks or warnings

  Scenario: A predicate programming error is not converted into idle fallback
    Given ordered cases configured for "predicate panic"
    When the application catches its configured predicate failure
    Then the original panic propagates and no candidate is admitted by fallback
    And no completed run or successful callback is invented
