Feature: Heads and tails

  # Attempts a single retry immediately.
  @retry
  Scenario: Tails
    Given a coin
    When I flip the coin
    Then I see tails

  # Attempts a single retry in 1 second.
  @retry.after(1s)
  Scenario: Heads
    Given a coin
    When I flip the coin
    Then I see heads

  # Attempts to retry 5 times with no delay between them.
  @retry(5)
  Scenario: Edge
    Given a coin
    When I flip the coin
    Then I see edge

  # Attempts to retry 10 times with 100 milliseconds delay between them.
  @retry(10).after(100ms)
  Scenario: Levitating
    Given a coin
    When I flip the coin
    Then the coin never lands
