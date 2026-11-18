@phase23
Feature: Applications attach typed evidence to actual execution scopes
  The runner supplies attachment identity. Text and binary evidence reaches
  official consumers under the step or scenario-hook attempt that produced it.
  Retaining a handle never grants permission to write into a later scope.

  Scenario: Text and binary evidence preserve caller content and metadata
    Given an attachment-aware application in "payloads" mode
    When its real execution is reported with the shared attachment store
    Then official Query retrieves exact text and binary evidence with their media types and file names
    And all typed attachments satisfy the pinned schema and execution reference graph

  Scenario: Scenario and step hooks attach within their actual execution scopes
    Given an attachment-aware application in "hooks" mode
    When its real execution is reported with the shared attachment store
    Then official Query retrieves evidence under the actual scenario hook or step
    And all typed attachments satisfy the pinned schema and execution reference graph

  Scenario: Concurrent scenarios retain independent attachment scopes
    Given an attachment-aware application in "concurrent" mode
    When its real execution is reported with the shared attachment store
    Then each concurrent scenario retrieves only its own evidence
    And all typed attachments satisfy the pinned schema and execution reference graph

  Scenario: Retried scenarios retain evidence from each distinct attempt
    Given an attachment-aware application in "retry" mode
    When its real execution is reported with the shared attachment store
    Then each attempt retrieves its own evidence and the final attempt passes
    And all typed attachments satisfy the pinned schema and execution reference graph

  Scenario: Handles retained across step boundaries cannot target a later step
    Given an attachment-aware application in "late step" mode
    When its real execution is reported with the shared attachment store
    Then the old step handle rejects the write while the current scope accepts evidence
    And all typed attachments satisfy the pinned schema and execution reference graph

  Scenario: Handles retained across retries cannot target a later attempt
    Given an attachment-aware application in "late attempt" mode
    When its real execution is reported with the shared attachment store
    Then the old attempt handle rejects the write while the retry accepts evidence
    And all typed attachments satisfy the pinned schema and execution reference graph

  Scenario: Completed applications cannot append evidence through retained handles
    Given an attachment-aware application in "completed" mode
    When its real execution is reported with the shared attachment store
    Then all retained scope handles reject writes after completion
    And all typed attachments satisfy the pinned schema and execution reference graph

  Scenario: Existing World implementations run without an attachment override
    Given an existing application with no attachment-aware constructor
    When its real execution is reported with the shared attachment store
    Then its original World constructor and steps still run successfully
    And the report contains no invented attachments

  Scenario: Lazy World construction can attach within its first actual step
    Given an attachment-aware application in "lazy" mode
    When its real execution is reported with the shared attachment store
    Then official Query retrieves constructor and body evidence under the first step
    And all typed attachments satisfy the pinned schema and execution reference graph
