Feature: Descriptions everywhere
  This is a single line description

  Scenario: two lines
  This description
  has two lines and indented with two spaces
    Given foo is 0

  Scenario: without indentation
  This is a description without indentation
    Given foo is 0

  Scenario: empty lines in the middle
  This description

  has an empty line in the middle
    Given foo is 0

  Scenario: empty lines around

  This description
  has an empty lines around

    Given foo is 0

  Scenario: comment after description
  This description
  has a comment after

# this is a comment
    Given foo is 0

  Scenario: comment right after description
  This description
  has a comment right after
    #  this is another comment

    Given foo is 0

  Scenario: description with escaped docstring separator
  This description has an \"\"\" (escaped docstring sparator)
    Given foo is 0
  Scenario Outline: scenario outline with a description
  This is a scenario outline description
    Given foo is <foo>
    Examples: examples with description
    This is an examples description
      | foo |
      | 0   |
