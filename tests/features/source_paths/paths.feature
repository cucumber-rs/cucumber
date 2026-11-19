@phase40
Feature: Discover an ordered selection of reusable feature sources
  Scenario: Preserve the supplied file sequence
    Given a source selection "files"
    When the selected source plan runs
    Then observed source cases are "B,A"
    And captured sources retain their exact original contents

  Scenario: Sort each directory expansion without sorting the full selection
    Given a source selection "directories"
    When the selected source plan runs
    Then observed source cases are "B,C,A"

  Scenario: Deduplicate overlapping selections at first occurrence
    Given a source selection "overlap"
    When the selected source plan runs
    Then observed source cases are "B,A,C"
    And each selected source constructs one World

  Scenario: Preserve sequence across explicit globs
    Given a source selection "globs"
    When the selected source plan runs
    Then observed source cases are "B,C,A"

  Scenario: An empty explicit selection executes no sources
    Given a source selection "empty"
    When the selected source plan runs
    Then observed source cases are ""

  Scenario: A missing literal file remains an observable parse error
    Given a source selection "missing"
    When the selected source plan runs
    Then observed source cases are "A"
    And one source error makes the run unsuccessful

  Scenario: An unmatched glob contributes no source
    Given a source selection "unmatched"
    When the selected source plan runs
    Then observed source cases are "A"

  Scenario: Syntax errors do not hide valid selected sources
    Given a source selection "invalid"
    When the selected source plan runs
    Then observed source cases are "A"
    And one source error makes the run unsuccessful

  Scenario: Source format still governs directory discovery
    Given a source selection "mixed formats"
    When the selected source plan runs
    Then observed source cases are "M,A"
    And captured sources retain their exact original contents

  Scenario: Source line selectors compose with the ordered selection
    Given a source selection "lines"
    When the selected source plan runs
    Then observed source cases are "B"

  Scenario: The explicit source plan takes precedence over the legacy CLI glob
    Given a source selection "cli override"
    When the selected source plan runs
    Then observed source cases are "B,A"

  Scenario: The default parser retains sorted directory discovery
    Given a source selection "legacy"
    When the selected source plan runs
    Then observed source cases are "A,B,C"

  Scenario: Sort valid Unicode filenames using the official string order
    Given a source selection "unicode paths"
    When the selected source plan runs
    Then observed source cases are "U,V"
