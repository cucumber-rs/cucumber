@phase29
Feature: Observe actual scenario callbacks without changing legacy lifecycle behavior
  Scoped callbacks and reporters can identify individual executions and inspect
  earlier cleanup failures while existing callbacks retain their original API.

  Scenario: Record ordered setup and reverse cleanup as separate actual callbacks
    Given an observed scenario-hook registry configured for "passing"
    When the application executes its scenario
    Then selected callbacks execute in registration order and reverse cleanup order
    And each executed callback has its own definition and invocation identity
    And recorded timestamps and outcomes describe actual executions

  Scenario: Failed setup records later selected setup as skipped
    Given an observed scenario-hook registry configured for "setup failure"
    When the application executes its scenario
    Then the failed setup callback and every cleanup callback execute
    And later selected setup is recorded as skipped without an invocation identity
    And no scenario body executes

  Scenario: Construction panics have the same cleanup guarantees as asynchronous failures
    Given an observed scenario-hook registry configured for "constructor panic"
    When the application executes its scenario
    Then every selected cleanup callback executes
    And the actual failing callback has its own diagnostic

  Scenario: Scoped cleanup observes earlier cleanup failures
    Given an observed scenario-hook registry configured for "cleanup failure"
    When the application executes its scenario
    Then the first cleanup callback observes a passing body
    And the next cleanup callback observes the earlier cleanup failure
    And both cleanup callbacks retain the original passing body snapshot

  Scenario: Equal-severity cleanup failures retain the original failing result
    Given an observed scenario-hook registry configured for "body and cleanup failure"
    When the application executes its scenario
    Then later cleanup retains the body's first failing diagnostic
    And every individual cleanup failure remains available in records

  Scenario: Existing after callbacks retain their pre-cleanup snapshot
    Given an observed scenario-hook registry configured for "mixed legacy and scoped"
    When the application executes its scenario
    Then legacy cleanup receives its original ScenarioFinished snapshot
    And scoped cleanup observes the earlier cleanup failure

  Scenario: Tag-excluded callbacks do not become skipped executions
    Given an observed scenario-hook registry selected by inherited feature Rule and scenario tags
    When the application executes its scenario
    Then only matching callback definitions have selected slots
    And excluded callbacks have neither skipped records nor invocation identities

  Scenario: Retrying preserves definitions and separates actual callback attempts
    Given an observed scenario-hook registry configured for "retry"
    When the application executes its scenario
    Then callback definitions remain stable across attempts
    And every attempt has a fresh World and distinct execution identities
    And scoped cleanup observes failure with retry availability only on the first attempt

  Scenario: Concurrent cases cannot exchange callback observations
    Given two concurrent scenarios sharing an observed scenario-hook registry
    When the application executes both scenarios
    Then each callback scope and record belongs to its actual scenario and attempt

  Scenario: Legacy stage replacement leaves no phantom callback executions
    Given an observed registry whose before stage is replaced using the legacy builder
    When the application executes its scenario
    Then only the legacy before callback and registered cleanup callbacks execute
    And replaced registrations have no selected slots or invocation records

  Scenario: Evidence belongs to the individual callback that produced it
    Given an observed scenario-hook registry configured for "attachments"
    When the application executes its scenario
    Then evidence from each callback retains that callback's definition identity
    And a handle retained from an earlier callback rejects later writes

  Scenario: Native empty-scenario behavior stays the default
    Given an observed registry with failing setup and an empty scenario under native policy
    When the application executes its scenario
    Then every selected cleanup callback still executes
    And scoped cleanup receives the native failed-setup snapshot

  Scenario: Explicit JS13.3 policy follows the empty-Pickle hook classification
    Given an observed registry with failing setup and an empty scenario under JS13.3 policy
    When the application executes its scenario
    Then registered cleanup slots are skipped without invented invocations
    And the initial setup failure remains the run failure

  Scenario: Empty-Pickle cleanup arguments remain absent under explicit JS13.3 policy
    Given an observed registry with passing setup and an empty scenario under JS13.3 policy
    When the application executes its scenario
    Then registered cleanup callbacks run in reverse order
    And their scoped result and retry-decision arguments are absent

  Scenario: Existing unobserved registries retain attachment reporting compatibility
    Given an unobserved legacy hook registry with typed attachments
    When the application executes its scenario
    Then its evidence remains under the native composite hook targets
    And a handle retained from an earlier callback rejects later writes

  Scenario: An observed stage can select zero callbacks
    Given an observed scenario-hook registry configured for "all excluded"
    When the application executes its scenario
    Then the plan records present stages with empty callback selections
    And no callback invocation is invented

  Scenario: World initialization failure is distinct from callback failure
    Given an observed registry whose World constructor fails
    When the application executes its scenario
    Then the constructor failure has its own observation
    And selected setup callbacks are skipped because initialization failed
    And selected cleanup callbacks still execute without a World

  Scenario: Constructor evidence is not attributed to a callback
    Given an observed scenario-hook registry configured for "constructor evidence"
    When the application executes its scenario
    Then constructor evidence retains its native setup target
    And callback evidence retains its individual callback target
