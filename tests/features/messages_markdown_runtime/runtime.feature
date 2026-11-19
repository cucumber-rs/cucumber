@phase28b
Feature: Explicit Markdown execution preserves exact captured source
  Scenario Outline: Official Markdown sources execute and report their exact graphs
    Given the runtime Markdown fixture "<fixture>"
    When it runs with explicit Markdown parsing and captured Messages
    Then all native handlers receive their Pickle text and arguments
    And its exact Markdown source and full official graph are reported
    And its completed run succeeds with valid execution references
    Examples:
      | fixture               |
      | minimal.feature.md    |
      | misc.feature.md       |
      | datatables.feature.md |
      | docstrings.feature.md |
      | tags.feature.md       |

  Scenario: Auto discovers ordinary and Markdown features without consuming unrelated documents
    Given a directory containing plain Markdown and unrelated sources
    When it runs with Auto source format
    Then exactly the plain and Markdown cases execute with their respective media types

  Scenario: Plain remains the default for custom source paths
    Given a classic feature at a custom extension
    When it runs with default source format
    Then its classic step executes and its source media type stays plain

  Scenario: Explicit Markdown accepts a caller-selected custom source path
    Given Markdown at a custom extension
    When it runs with explicit Markdown parsing and captured Messages
    Then its Markdown step executes and its source media type stays Markdown

  Scenario: Auto rejects an unrecognized source extension visibly
    Given Markdown at a custom extension
    When it runs with Auto source format
    Then a format failure prevents successful completion

  Scenario: Invalid Markdown retains original source and correct error media
    Given an invalid Markdown source without a final newline
    When it runs with explicit Markdown parsing and captured Messages
    Then its parse error reports the captured Markdown bytes and original location

  Scenario: Captured Markdown survives deletion before execution
    Given a captured Markdown outline deleted after parsing
    When its recorded native cases execute with captured Messages
    Then all native handlers receive their Pickle text and arguments
    And the original Markdown graph and source remain available without rereading
    And its completed run succeeds with valid execution references

  Scenario: Empty Markdown documents report no synthetic Feature or cases
    Given an empty Markdown source
    When it runs with explicit Markdown parsing and captured Messages
    Then an exact empty Markdown source document completes successfully

  Scenario: A globally selected dialect controls Markdown runtime interpretation
    Given a French Markdown source with English-looking language prose
    When it runs with explicit Markdown parsing and captured Messages
    Then French steps execute and the reported graph remains French

  Scenario: Typed Markdown snapshots preserve earlier generations
    Given the runtime Markdown fixture "docstrings.feature.md"
    When snapshots capture Markdown then plain then invalid Markdown at the same path
    Then retained typed Markdown metadata and bytes remain unchanged
