@phase18
Feature: Native tracing logs remain faithful in Messages reports
  Logs retain exactly the body and scenario-attempt identity observed by the
  runner. Missing step identity is not inferred from misleading lifecycle order.

  Scenario Outline: Real tracing logs survive reporting without changed attribution
    Given a native tracing application in "<mode>" mode
    When the application reports through Messages
    Then every observed native log has one exact case-attempt attachment
    And log attachments have no inferred step or hook identity
    And the "<mode>" run retains its expected logs and final outcome
    And all log report envelopes satisfy the pinned Messages schema

    Examples:
      | mode       |
      | plain      |
      | hooks      |
      | retry      |
      | concurrent |
      | fanout     |

  @phase37a_messages
  Scenario Outline: Ordered retries retain their actual trace and consumer identities
    Given a native tracing application in "<mode>" mode
    When the application reports through Messages
    Then every observed native log has one exact case-attempt attachment
    And log attachments have no inferred step or hook identity
    And the "<mode>" run retains its expected logs and final outcome
    And all log report envelopes satisfy the pinned Messages schema
    And official Query resolves every ordered attempt and its log identities
    And the ordered retry clock records <delays> delays

    Examples:
      | mode                | delays |
      | ordered retries     | 0      |
      | ordered retry delay | 2      |
