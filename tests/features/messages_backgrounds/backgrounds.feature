@phase16
Feature: Empty scenarios have an explicit background execution policy
  Portable official Pickles contain no background steps for an empty scenario.
  Applications can opt into matching execution without losing scenario hooks.

  Scenario Outline: Official execution matches empty and nonempty Pickles
    Given a "<shape>" feature with background steps
    When it executes with the Gherkin 43 background policy and Messages
    Then its complete AST and Pickles match the pinned official compiler
    And empty Pickles execute only their scenario hook stages
    And real handlers ran <backgrounds> background steps and <bodies> body steps
    And the background report is schema-valid and successful
    Examples:
      | shape            | backgrounds | bodies |
      | feature empty    | 0           | 0      |
      | rule empty       | 0           | 0      |
      | feature outline  | 0           | 0      |
      | rule outline     | 0           | 0      |
      | mixed            | 3           | 2      |
      | no background    | 0           | 0      |
      | official fixture | 0           | 0      |

  Scenario: Runner clones and type-changing builders retain the selected policy
    Given a "rule empty" feature with background steps
    When a cloned configured runner executes after hook and scheduling builders
    Then its complete AST and Pickles match the pinned official compiler
    And empty Pickles execute only their scenario hook stages
    And real handlers ran 0 background steps and 0 body steps
    And the background report is schema-valid and successful

  Scenario Outline: Legacy applications retain background execution
    Given a "<shape>" feature with background steps
    When it executes with the default runner and native summary writer
    Then real handlers ran <backgrounds> background steps and 0 body steps
    And the native run has no failures or parser errors
    Examples:
      | shape         | backgrounds |
      | feature empty | 1           |
      | rule empty    | 2           |

  Scenario: Legacy execution cannot masquerade as an official empty Pickle
    Given a "rule empty" feature with background steps
    When it executes with the default runner and source-backed Messages
    Then reporting visibly rejects the unexpected background execution
    And no successful empty-scenario completion is emitted

  Scenario: Source-only conversion needs no runner policy
    Given a "rule outline" feature with background steps
    When its legacy parser snapshot is converted without execution
    Then its complete AST and Pickles match the pinned official compiler
    And its empty Pickles retain feature and Rule backgrounds only in the AST
