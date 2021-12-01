# Cucumber

![Test](rec/test.gif)

[Cucumber] is a specification for running tests in a behavioral driven development style workflow ([BDD](https://en.wikipedia.org/wiki/Behavior-driven_development)). It assumes involvement of non-technical members on a project and as such provides a human-readable syntax for the definition of features, via the language [Gherkin]. A typical feature could look something like this:

```gherkin
Feature: User login

  Scenario: User tries to log in with an incorrect password
    Given a user portal where one can login
    When a user types in the correct username but incorrect password
    Then the user will see a messagebox with an alert that their password is wrong
```

These features are agnostic to the implementation, the only requirement is that they follow the expected format of phrases followed by the keywords (`Given`, `When`, `Then`). 

[Gherkin] offers support for languages other than English, as well. [Cucumber] implementations then simply hook into these keywords and execute the logic that corresponds to the keywords. [`cucumber-rust`] is one of such implementations and is the subject of this book.

```rust,ignore
#[given("a user portal where one can login")]
fn portal(w: &mut World) {
    /* initial setup of the feature being tested */
}

#[when("a user types in the correct username but incorrect password")]
fn incorrect_password(w: &mut World) {
    /* performing the relevant actions against the feature */
}

#[then("the user will see a messagebox with an alert that their password is wrong")]
fn alert(w: &mut World) {
    /* assertions and validation that the feature is working as intended */
}
```

Since the goal is the testing of externally identifiable behavior of some feature, it would be a misnomer to use [Cucumber] to test specific private aspects or isolated modules. [Cucumber] tests are more likely to take the form of integration, functional or E2E testing.




[Cucumber]: https://cucumber.io
[Gherkin]: https://cucumber.io/docs/gherkin/reference
[`cucumber-rust`]: https://docs.rs/cucumber-rust
