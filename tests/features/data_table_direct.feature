Feature: Direct DataTable parameters in steps

  Scenario: Using required DataTable parameter
    Given the following products exist
      | name   | price | stock |
      | apple  | 1.50  | 100   |
      | banana | 0.75  | 200   |
      | orange | 2.00  | 150   |
    When I add items to cart
      | product | quantity |
      | apple   | 3        |
      | banana  | 5        |
    Then the order summary should contain
      | product | quantity | total |
      | apple   | 3        | 4.50  |
      | banana  | 5        | 3.75  |

  Scenario: Using optional DataTable parameter
    Given the following products exist
      | name  | price | stock |
      | apple | 1.50  | 100   |
    When I add items to cart
    # No table provided - should add all products with quantity 1
    Then the order summary should contain
      | product | quantity | total |
      | apple   | 1        | 1.50  |

  Scenario: DataTable with captured parameters
    Given the following products exist
      | name   | price | stock |
      | apple  | 10.00 | 100   |
      | banana | 5.00  | 200   |
      | orange | 8.00  | 150   |
    When I add items to cart
      | product | quantity |
      | apple   | 2        |
      | banana  | 4        |
      | orange  | 1        |
    And I apply a 20% discount with exclusions
      | excluded_product |
      | apple           |
    # apple: 2 * 10 = 20 (no discount)
    # banana: 4 * 5 * 0.8 = 16
    # orange: 1 * 8 * 0.8 = 6.4
    # Total: 42.4

  Scenario: Using rows_hash for configuration
    Given the store configuration
      | tax_rate      | 0.08  |
      | minimum_order | 10.00 |
      | currency      | USD   |
    And the following products exist
      | name  | price | stock |
      | apple | 5.00  | 100   |

  Scenario: Using transposed tables
    Given the following products exist
      | name   | price | stock |
      | apple  | 1.50  | 10    |
      | banana | 0.75  | 20    |
    When I process transposed inventory
      | apple | banana |
      | 15    | 25     |
    # Stock should be updated: apple=25, banana=45

  Scenario: Using column selection
    Given the following products exist
      | name   | price | stock | category |
      | apple  | 1.50  | 100   | fruit    |
      | carrot | 0.50  | 200   | vegetable |
    When I add items to cart
      | product | quantity | note            |
      | apple   | 2        | for pie         |
      | carrot  | 5        | for soup        |
    Then the order summary should contain
      | product | quantity | total | note     |
      | apple   | 2        | 3.00  | for pie  |
      | carrot  | 5        | 2.50  | for soup |