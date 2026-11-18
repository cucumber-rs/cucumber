@sdk46_negative
Feature: SDK acceptance rejects misleading package and evidence results

  @SDK-001
  Scenario: Reject false acceptance for compile plaintext source to faithful ast and pickles in all three installed sdks
    Given the installed native SDK contract "SDK-001"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-002
  Scenario: Reject false acceptance for compile markdown through the same native implementation
    Given the installed native SDK contract "SDK-002"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-003
  Scenario: Reject false acceptance for compile empty/comment-only input without invented scenarios
    Given the installed native SDK contract "SDK-003"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-004
  Scenario: Reject false acceptance for recover after malformed input in the same host
    Given the installed native SDK contract "SDK-004"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-005
  Scenario: Reject false acceptance for discover the versioned implemented operations
    Given the installed native SDK contract "SDK-005"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-006
  Scenario: Reject false acceptance for derive a lossless rerun plan from real messages
    Given the installed native SDK contract "SDK-006"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-007
  Scenario: Reject false acceptance for reject incomplete or inconsistent run evidence
    Given the installed native SDK contract "SDK-007"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-008
  Scenario: Reject false acceptance for represent a completed passing run as no rerun
    Given the installed native SDK contract "SDK-008"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-009
  Scenario: Reject false acceptance for extend the safe sdk from an external rust consumer
    Given the installed native SDK contract "SDK-009"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-010
  Scenario: Reject false acceptance for consume actual native package artifacts outside the checkout
    Given the installed native SDK contract "SDK-010"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence

  @SDK-011
  Scenario: Reject false acceptance for keep requests isolated in one host process
    Given the installed native SDK contract "SDK-011"
    When its declared negative control is exercised
    Then the false result is rejected without losing original evidence
