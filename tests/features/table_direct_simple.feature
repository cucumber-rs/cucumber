Feature: Simple direct DataTable test

  Scenario: Direct DataTable parameter
    Given the following items
      | name  | value |
      | apple | 1     |
      | banana| 2     |
      
  Scenario: Optional DataTable with table
    Given optional items
      | name  | value |
      | orange| 3     |
      
  Scenario: Optional DataTable without table
    Given optional items