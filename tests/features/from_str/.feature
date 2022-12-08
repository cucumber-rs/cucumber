Feature: FromStr
  Scenario: FromStr
    Given regex: int: 42
    And expr: int: 42
    And regex: quoted: 'inner'
    And expr: quoted: 'inner'
