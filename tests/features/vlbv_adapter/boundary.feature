@phase39_vlbv
Feature: Project real Cucumber evidence into the existing VLBV boundary result contract
  Scenario: Complete real-path receipts support scoped acceptance
    Given a real Cucumber run with frozen v2 contract and complete assertion control and cleanup receipts
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "PASS"
    And the canonical result fields and all original evidence references are retained

  Scenario: A green runner is insufficient without an exercised oracle
    Given boundary evidence for "green without assertion"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "NOT_EXERCISED"

  Scenario: An exercised product mismatch remains a failure
    Given boundary evidence for "product mismatch"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "FAIL"
    And the actual failed observation remains in the attempt ledger

  Scenario Outline: Wrong or stale identities cannot support acceptance
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    Examples:
      | fixture                 |
      | wrong candidate         |
      | stale production source |
      | wrong contract binding  |
      | wrong canonical run     |

  Scenario Outline: Unexercised application paths never become acceptance
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "NOT_EXERCISED"
    Examples:
      | fixture        |
      | skipped step   |
      | dry run        |
      | undefined step |

  Scenario: The adopted denominator includes scenarios absent from the runner
    Given boundary evidence for "missing adopted scenario"
    When the read-only Messages converter projects the boundary
    Then the exercised scenario passes and the absent adopted scenario is NOT_EXERCISED

  Scenario Outline: Every material oracle family needs an effective negative control
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "NOT_EXERCISED"
    Examples:
      | fixture               |
      | missing control       |
      | non-sensitive control |

  Scenario: Unknown restoration blocks acceptance
    Given boundary evidence for "unknown cleanup"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"

  Scenario Outline: Unusable execution evidence is not a product acceptance claim
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    Examples:
      | fixture          |
      | process exit two |
      | truncated stream |

  Scenario: A successful retry retains the earlier failed oracle
    Given boundary evidence for "failed then successful retry"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "FAIL"
    And every failed and successful attempt is retained

  Scenario Outline: Repair authority comes from existing bound authorization records
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "<verdict>"
    And every failed and successful attempt is retained
    Examples:
      | fixture                | verdict |
      | unauthorized repair    | BLOCKED |
      | bound authorized repair| PASS    |

  Scenario: An unsupported acceptance profile fails closed
    Given boundary evidence for "stochastic profile"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"

  Scenario: Projection never writes canonical state or launches the boundary packet
    Given boundary evidence for "read-only projection"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "PASS"
    And every canonical input and sentinel remains unchanged

  Scenario Outline: Passing bound steps cannot hide incomplete or unclassified run evidence
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "<reason>"
    Examples:
      | fixture                   | reason                    |
      | nonzero green process     | nonzero product process   |
      | failed global completion  | unsuccessful run          |
      | unknown step status       | unknown step status       |
      | unmapped failing attempt  | unmapped failing attempt  |
