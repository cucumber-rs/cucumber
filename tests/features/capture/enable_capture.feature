Feature: The enable_capture flag controls whether stdout/stderr printing is captured or not
  Scenario: In which some printing is attempted
    When we print "everything is great" to stdout
    And we print "something went wrong" to stderr
    Then it is up to the cucumber configuration to decide whether the content gets printed
