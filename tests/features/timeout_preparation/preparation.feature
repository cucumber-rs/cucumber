@phase30c_preparation
Feature: Explicit preparation boundaries preserve existing native callback timing
  Scenario Outline: Existing macro registrations retain lazy direct invocation
    Given an existing "<macro>" definition without preparation opt-in
    When its registered callback is constructed and then first polled
    Then argument conversion and the handler body first occur during polling
    Examples:
      | macro   |
      | regex   |
      | expr    |
      | expr_js |

  Scenario Outline: Opted-in owned preparation precedes the asynchronous deadline
    Given an opted-in "<fixture>" definition with before-deadline callback construction
    When its application executes with a recording timeout clock
    Then owned argument preparation precedes timer construction
    And its handler receives the existing converted values
    And borrowed arguments remain valid across suspension
    Examples:
      | fixture                  |
      | custom scalar conversion |
      | JavaScript quoted string |
      | slice and borrowed step  |

  Scenario: Preparation failure keeps normal cleanup without installing a body timer
    Given an opted-in definition whose argument conversion fails
    When its application executes with a recording timeout clock
    Then the existing step failure identifies argument conversion
    And no body timer or handler body starts
    And normal cleanup still runs

  Scenario Outline: Raw callback construction follows the explicitly selected boundary
    Given a raw callback with "<boundary>" timeout construction policy
    When its application executes with a recording timeout clock
    Then the callback factory and timer occur in "<order>" order
    And expiry still cancels its pending future and runs cleanup
    Examples:
      | boundary       | order           |
      | WithinDeadline | timer factory   |
      | BeforeDeadline | factory timer   |

  Scenario: Before-deadline factory panic remains an ordinary caught failure
    Given a raw callback whose before-deadline factory panics
    When its application executes with a recording timeout clock
    Then the existing step failure identifies the factory panic
    And no body timer or handler body starts
    And normal cleanup still runs

  Scenario Outline: Hook callback factories use the same explicit boundary
    Given a "<hook>" callback with before-deadline construction policy
    When its application executes with a recording timeout clock
    Then the selected hook factory precedes its timer
    And its established lifecycle completion and cleanup remain intact
    Examples:
      | hook            |
      | before scenario |
      | after scenario  |
      | before step     |
      | after step      |
      | before run      |
      | after run       |

  Scenario: Disabled limits preserve invocation without a timer
    Given an opted-in definition with a disabled timeout
    When its application executes with a recording timeout clock
    Then argument preparation and the handler run normally without any timer
