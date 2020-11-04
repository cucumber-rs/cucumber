Feature: Failing

    Scenario: Step that succeeds should return an Ok
        Given nothing

    Scenario: Step that fails should return an Err
        Given a panic
