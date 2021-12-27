Feature: Animal feature

  Scenario: If we feed a hungry Felix it will no longer be hungry
    Given a hungry cat
      """
      A hungry cat called Felix is rescued from a Whiskas tin in a calamitous
      mash-up of cat food brands.
      """
    When I feed the cat
    Then the cat is not hungry
