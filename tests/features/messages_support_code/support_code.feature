@phase27_support_code
Feature: Actual registered support code is discoverable in standard Messages
  Scenario: Consumers resolve a macro definition to its actual Rust source
    Given a support-code run configured as "macro"
    When that run completes and the pinned official Query reads its Messages
    Then the consumer resolves the actual compiled regex and Rust registration location
    And every emitted envelope satisfies the pinned Messages schema

  Scenario Outline: Definition references follow the selected matching policy
    Given a support-code run configured as "<mode>"
    When that run completes and the pinned official Query reads its Messages
    Then the consumer resolves <matches> definitions and observes "<status>"
    And every emitted envelope satisfies the pinned Messages schema
    Examples:
      | mode                 | matches | status    |
      | unique               | 1       | PASSED    |
      | ambiguous            | 2       | AMBIGUOUS |
      | undefined            | 0       | UNDEFINED |
      | keyword-specific     | 0       | UNDEFINED |
      | keyword-independent  | 1       | PASSED    |
      | independent-late     | 1       | PASSED    |
      | cross-keyword        | 2       | AMBIGUOUS |

  Scenario: Later collection builders replace the observed registry and its policy
    Given a support-code run configured as "replacement"
    When that run completes and the pinned official Query reads its Messages
    Then only the final replacement collection is reported and executed
    And every emitted envelope satisfies the pinned Messages schema

  Scenario: Duplicate raw registrations retain existing replacement semantics
    Given a support-code run configured as "duplicate"
    When that run completes and the pinned official Query reads its Messages
    Then the consumer resolves one raw definition with an empty source reference
    And every emitted envelope satisfies the pinned Messages schema

  Scenario: Definition identities remain stable across retries
    Given a support-code run configured as "retry"
    When that run completes and the pinned official Query reads its Messages
    Then both attempts reference the same single registered definition
    And every emitted envelope satisfies the pinned Messages schema

  Scenario: Custom runners may omit support-code metadata without implementing a new method
    Given a support-code run configured as "custom"
    When that run completes and the pinned official Query reads its Messages
    Then unsupported definition metadata is omitted rather than fabricated
    And every emitted envelope satisfies the pinned Messages schema

  Scenario: Definitions remain discoverable for planned steps skipped after a failure
    Given a support-code run configured as "skipped"
    When that run completes and the pinned official Query reads its Messages
    Then the skipped planned step still resolves its actual definition
    And every emitted envelope satisfies the pinned Messages schema

  Scenario: Type-changing runner builders preserve the reporting handoff
    Given a support-code run configured as "builder-chain"
    When that run completes and the pinned official Query reads its Messages
    Then the consumer resolves 1 definitions and observes "PASSED"
    And every emitted envelope satisfies the pinned Messages schema
