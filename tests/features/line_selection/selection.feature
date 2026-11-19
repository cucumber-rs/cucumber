@phase34
Feature: Source-aware line selection uses original declarations and Examples rows
  Scenario Outline: Select only source locations linked to an executable scenario
    Given a "<format>" selection source with comments duplicate rows and Rule backgrounds
    And its selected source locations are "<locations>"
    When the source executes with explicit line selection
    Then the selected scenario bodies are "<expected>"
    And origin metadata retains every original scenario and Examples row
    Examples:
      | format   | locations          | expected                         |
      | plain    | scenario           | health                           |
      | plain    | outline            | invoice green, invoice green, invoice red |
      | plain    | first row          | invoice green                    |
      | plain    | second row         | invoice green                    |
      | plain    | last row           | invoice red                      |
      | plain    | first and last row | invoice green, invoice red       |
      | plain    | feature            |                                  |
      | plain    | rule               |                                  |
      | plain    | background         |                                  |
      | plain    | step               |                                  |
      | plain    | examples           |                                  |
      | plain    | table header       |                                  |
      | plain    | comment            |                                  |
      | markdown | scenario           | health                           |
      | markdown | outline            | invoice green, invoice green, invoice red |
      | markdown | first row          | invoice green                    |
      | markdown | second row         | invoice green                    |
      | markdown | last row           | invoice red                      |
      | markdown | first and last row | invoice green, invoice red       |
      | markdown | feature            |                                  |
      | markdown | rule               |                                  |
      | markdown | background         |                                  |
      | markdown | step               |                                  |
      | markdown | examples           |                                  |
      | markdown | table header       |                                  |
      | markdown | separator          |                                  |

  Scenario: Source lines compose with typed names tags and the application predicate
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And its selected source locations are "outline"
    And typed names tags and the caller predicate select only the eligible green invoice
    When the source executes with explicit line selection
    Then the selected scenario bodies are "invoice green, invoice green"

  Scenario: Repeated source selections are a union without duplicate execution
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And the first row is selected repeatedly by absolute relative and symlink paths
    When the source executes with explicit line selection
    Then the selected scenario bodies are "invoice green"

  Scenario: An unrestricted file selection supersedes its restricted lines
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And the source is selected by its first row and as a whole file
    When the source executes with explicit line selection
    Then the selected scenario bodies are "health, invoice green, invoice green, invoice red"

  Scenario: Multiple discovered files are selected independently
    Given two discoverable sources and a selector for an absent third source
    When each discovered source is selected by its scenario declaration
    Then both selected files execute once and the absent selector causes no error

  Scenario: A selector outside discovery does not add new inputs
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And only a path outside parser discovery is selected
    When the source executes with explicit line selection
    Then the selected scenario bodies are ""

  Scenario: Empty selectors impose no additional restriction
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And an empty explicit source selection
    When the source executes with explicit line selection
    Then the selected scenario bodies are "health, invoice green, invoice green, invoice red"

  Scenario: Invalid selector values are rejected before execution
    Given an unconfigured source selector
    When a zero line and an empty path are requested
    Then both invalid values are rejected with meaningful errors

  Scenario: Captured origin generations survive changes and deletion
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    When a source snapshot is retained then its file is changed reparsed and deleted
    Then retained origins still refer to their original source generation

  Scenario: Default parsing retains legacy expansion without source selection
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    When the source executes without line selectors
    Then the selected scenario bodies are "health, invoice green, invoice green, invoice red"

  @phase34_messages
  Scenario: Selecting a duplicate row preserves the complete source graph
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And its selected source locations are "second row"
    When the source executes with source-backed Messages and line selection
    Then the selected scenario bodies are "invoice green"
    And the report retains all Pickles but references only the selected original row

  Scenario: Line-only convenience also composes a configured CLI name and caller predicate
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And its selected source locations are "outline"
    And the CLI selects red while the caller selects green
    When the source executes with explicit line selection
    Then the selected scenario bodies are ""

  Scenario: Line selection captures its own metadata without a reporting store
    Given a "plain" selection source with comments duplicate rows and Rule backgrounds
    And its selected source locations are "first row"
    And no external source store is configured
    When the source executes with explicit line selection
    Then the selected scenario bodies are "invoice green"
