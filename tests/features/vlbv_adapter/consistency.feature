@phase43_vlbv
Feature: Receipt consistency prevents malformed evidence from becoming acceptance
  Scenario Outline: Every lifecycle invocation has a causally complete record
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "<reason>"
    Examples:
      | fixture                   | reason                  |
      | orphan step start         | orphan step start       |
      | orphan global hook finish | global hook             |
      | unfinished global hook    | global hook             |
      | reversed step lifecycle   | lifecycle order         |

  Scenario Outline: Retry history cannot omit or invent an invocation
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "retry chain"
    Examples:
      | fixture                  |
      | nonzero first attempt    |
      | duplicate attempt number |
      | missing promised retry   |
      | unpromised successor     |

  Scenario: Repair history must have a distinct previously admitted predecessor
    Given boundary evidence for "self repair predecessor"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "repair predecessor"

  Scenario Outline: A green summary cannot conceal a recorded global failure
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "<reason>"
    Examples:
      | fixture                   | reason             |
      | concealed parse error     | parse error        |
      | concealed global failure  | global hook failed |
      | concealed run exception   | run exception      |

  Scenario Outline: Process receipts bind the exact product and control streams
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "Messages artifact binding"
    And a legacy receipt without the stream binding is also blocked with migration guidance
    Examples:
      | fixture                   |
      | substituted product stream|
      | substituted control stream|

  Scenario: Valid interleaved scenarios keep their independent receipt histories
    Given boundary evidence for "valid interleaved lifecycle"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "PASS"
    And all interleaved adopted scenarios pass with separate attempt identities

  Scenario Outline: Explicit receipts cannot be ambiguous or silently ignored
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "<reason>"
    Examples:
      | fixture                    | reason              |
      | nonboolean exercised claim | exercised boolean   |
      | orphan failed assertion    | orphan assertion    |
      | conflicting cleanup        | duplicate cleanup   |
      | conflicting control        | duplicate control   |

  Scenario Outline: A supplied receipt cannot disappear behind an unexercised or failed earlier attempt
    Given boundary evidence for "<fixture>"
    When the read-only Messages converter projects the boundary
    Then the adopted scenario verdict is "BLOCKED"
    And the refusal identifies "<reason>"
    Examples:
      | fixture                              | reason                    |
      | orphan control assertion             | orphan control assertion  |
      | repair after unexercised cleanup      | cleanup unknown           |
      | repair after inconsistent control     | Messages artifact binding |
