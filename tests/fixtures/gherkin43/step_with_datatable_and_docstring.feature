Feature: Step with DataTable and DocString

  Scenario: DataTable followed by DocString
    Given a step with both arguments
      | id | name |
      | 1  | bob  |
      """
      hello
      """

  Scenario: DocString followed by DataTable
    Given a step with both arguments
      """
      hello
      """
      | id | name |
      | 1  | bob  |
