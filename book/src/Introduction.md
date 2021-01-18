# Cucumber

[Cucumber](https://cucumber.io/) is a specification for running tests in a behavioral driven development style workflow ([BDD](https://en.wikipedia.org/wiki/Behavior-driven_development)). It assumes involvement of non-technical members on a project and as such provides a human-readable syntax for the definition of features, via the langauge [Gherkin](https://cucumber.io/docs/gherkin/reference/). A typical feature could look something like this:

```
Feature: User login

  Scenario: User tries to log in with an incorrect password
    Given a user portal where one can login
    When a user types in the correct username but incorrect password
    Then the user will see a messagebox with an alert that their password is wrong
```

These features are agnostic to the implementation, the only requirement is that they follow the expected format of phrases followed by the keywords ("Given", "When", "Then"). Gherkin offers support for languages other than English, as well.

Cucumber implementations then simply hook into these keywords and execute the logic that corresponds to the keywords. `cucumber-rust` is one such implementation and is the subject of this book.

```
.given("a user portal where one can login",
    /* initial setup of the feature being tested */
)
.when("a user types in the correct username but incorrect password",
    /* performing the relevant actions against the feature */
)
.then("the user will see a messagebox with an alert that their password is wrong", 
    /* assertions and validation that the feature is working as intended */
)
```

Since the goal is the testing of externally identifiable behavior of some feature, it would be a misnomer to use Cucumber to test specific private aspects or isolated modules. Cucumber tests are more likely to take the form of integration or functional testing.
