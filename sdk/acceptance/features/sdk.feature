@sdk46
Feature: Applications share the native Rust source and evidence SDK

  @SDK-001
  Scenario: Compile plaintext source to faithful AST and Pickles in all three installed SDKs
    Given the installed native SDK contract "SDK-001"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-002
  Scenario: Compile Markdown through the same native implementation
    Given the installed native SDK contract "SDK-002"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-003
  Scenario: Compile empty/comment-only input without invented scenarios
    Given the installed native SDK contract "SDK-003"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-004
  Scenario: Recover after malformed input in the same host
    Given the installed native SDK contract "SDK-004"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-005
  Scenario: Discover the versioned implemented operations
    Given the installed native SDK contract "SDK-005"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-006
  Scenario: Derive a lossless rerun plan from real Messages
    Given the installed native SDK contract "SDK-006"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-007
  Scenario: Reject incomplete or inconsistent run evidence
    Given the installed native SDK contract "SDK-007"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-008
  Scenario: Represent a completed passing run as no rerun
    Given the installed native SDK contract "SDK-008"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-009
  Scenario: Extend the safe SDK from an external Rust consumer
    Given the installed native SDK contract "SDK-009"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-010
  Scenario: Consume actual native package artifacts outside the checkout
    Given the installed native SDK contract "SDK-010"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract

  @SDK-011
  Scenario: Keep requests isolated in one host process
    Given the installed native SDK contract "SDK-011"
    When the application exercises its real package boundary
    Then the result satisfies its independent source contract
