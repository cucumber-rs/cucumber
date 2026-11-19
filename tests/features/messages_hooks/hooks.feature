@phase11
Feature: Report observed native scenario hook stages through Messages
  Consumers need setup and cleanup outcomes attached to the right execution,
  without invented callback identities or source locations.

  Scenario Outline: Composite setup and cleanup preserve the real outcome
    Given a Messages hook run configured as "<config>"
    When the hooked feature executes with retry budget 0
    Then hook attempt statuses are "<statuses>"
    And hook run success is "<success>"
    And the callback trace is "<trace>"
    And the report describes only the two native composite stages
    And hook envelopes satisfy the pinned schema and ordered reference graph
    Examples:
      | config         | statuses                     | success | trace                             |
      | passing        | PASSED,PASSED,PASSED,PASSED   | true    | before-one,before-two,body,after-two,after-one |
      | before-failure | FAILED,SKIPPED,SKIPPED,PASSED | false   | before-one,after-two,after-one     |
      | after-failure  | PASSED,PASSED,PASSED,FAILED   | false   | before-one,before-two,body,after-two,after-one |
      | body-failure   | PASSED,FAILED,SKIPPED,PASSED  | false   | before-one,before-two,body,after-two,after-one |
      | undefined      | PASSED,UNDEFINED,SKIPPED,PASSED | false | before-one,before-two,after-two,after-one |
      | filtered       | PASSED,PASSED,PASSED,PASSED   | true    | body                              |
      | empty          | PASSED,PASSED                 | true    | before-one,before-two,after-two,after-one |

  Scenario Outline: Retry attempts reuse stage identities and rerun cleanup
    Given a Messages hook run configured as "<config>"
    When the hooked feature executes with retry budget 1
    Then hook attempt statuses are "<statuses>"
    And hook run success is "<success>"
    And the report describes only the two native composite stages
    And retries retain one test case and stage identities with flags "true,false"
    And hook envelopes satisfy the pinned schema and ordered reference graph
    Examples:
      | config         | statuses                                                  | success |
      | before-retry   | FAILED,SKIPPED,SKIPPED,PASSED;PASSED,PASSED,PASSED,PASSED    | true    |
      | after-retry    | PASSED,PASSED,PASSED,FAILED;PASSED,PASSED,PASSED,PASSED      | true    |
      | after-failure  | PASSED,PASSED,PASSED,FAILED;PASSED,PASSED,PASSED,FAILED      | false   |
