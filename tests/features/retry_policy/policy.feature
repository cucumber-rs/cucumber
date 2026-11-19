@phase36
Feature: Validate an explicit retry budget and official inherited tag gate
  Applications can choose retry compatibility without native magic tags or
  hidden legacy CLI overrides. Eligibility remains an explicit outcome policy.

  Scenario: A typed tag gate requires a positive budget before execution
    Given retry budget 0 and an explicitly supplied official tag gate
    When the application validates its retry policy
    Then validation rejects the configuration before parsing or constructing a World

  Scenario: Zero without a gate explicitly disables retry
    Given an application with retry policy "disabled"
    When the application runs with explicit JS outcome eligibility
    Then exactly 1 attempt is reported and its final status is "FAILED"

  Scenario Outline: Official matching includes every inherited tag location
    Given an application with retry policy "<location>"
    When the application runs with explicit JS outcome eligibility
    Then exactly 2 attempts are reported and its final status is "PASSED"
    And each attempt owns a fresh World and cleanup completes

    Examples:
      | location         |
      | feature tag      |
      | Rule tag         |
      | scenario tag     |
      | Examples tag     |

  Scenario: A native retry tag cannot bypass an unmatched official gate
    Given an application with retry policy "unmatched magic tag"
    When the application runs with explicit JS outcome eligibility
    Then exactly 1 attempt is reported and its final status is "FAILED"

  Scenario: A native retry tag cannot enlarge the explicit budget
    Given an application with retry policy "matched magic tag"
    When the application runs with explicit JS outcome eligibility
    Then exactly 2 attempts are reported and its final status is "FAILED"

  Scenario: Explicit typed policy is authoritative over legacy CLI retry configuration
    Given an application with retry policy "conflicting legacy CLI"
    When the application runs with explicit JS outcome eligibility
    Then exactly 2 attempts are reported and its final status is "FAILED"

  Scenario: A later custom retry-options callback replaces the policy callback
    Given an application with retry policy "custom override"
    When the application runs with explicit JS outcome eligibility
    Then exactly 1 attempt is reported and its final status is "FAILED"

  Scenario Outline: A configured budget does not make non-failed outcomes retryable
    Given an application with retry policy "<outcome>"
    When the application runs with explicit JS outcome eligibility
    Then exactly 1 attempt is reported and its final status is "<status>"

    Examples:
      | outcome    | status    |
      | pending    | PENDING   |
      | ambiguous  | AMBIGUOUS |
      | undefined  | UNDEFINED |
      | skipped    | SKIPPED   |

  Scenario Outline: Protocol attempts agree with recovered and exhausted execution
    Given an application with retry policy "<mode>"
    When the application runs with explicit JS outcome eligibility
    Then exactly <attempts> attempts are reported and its final status is "<status>"
    And schema-valid Messages and official Query retain the final success "<success>"
    And each attempt owns a fresh World and cleanup completes

    Examples:
      | mode      | attempts | status | success |
      | recovered | 2        | PASSED | true    |
      | exhausted | 3        | FAILED | false   |
