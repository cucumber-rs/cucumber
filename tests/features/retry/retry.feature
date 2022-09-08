Feature: retries

  Scenario: no tags
    Given fail 3 times

  @serial
  Scenario: serial tag
    Given fail 3 times

  @serial @flaky
  Scenario: serial and flaky tags
    Given fail 3 times

  @retry
  Scenario: retry tag
    Given fail 3 times

  @retry(2)
  Scenario: explicit number of retries
    Given fail 3 times

  @retry.after(1s)
  Scenario: explicit retry timeout
    Given fail 3 times

  @retry(2).after(1s)
  Scenario: explicit number of retries and timeout
    Given fail 3 times
