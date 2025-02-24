Introduction
============

[Cucumber] is a specification for running tests in a [BDD] (behavior-driven development) style workflow. 

It assumes involvement of non-technical members on a project and as such provides a human-readable syntax for the definition of features, via the language [Gherkin]. A typical feature could look something like this:
```gherkin
Feature: Eating too much cucumbers may not be good for you
    
  Scenario: Eating a few isn't a problem
    Given Alice is hungry
    When she eats 3 cucumbers
    Then she is full
```

These features are agnostic to the implementation, the only requirement is that they follow the expected format of phrases followed by the keywords (`Given`, `When`, `Then`). [Gherkin] offers support for [languages other than English][1], as well.

[Cucumber] implementations then simply hook into these keywords and execute the logic corresponding to the keywords. [`cucumber`] crate is one of such implementations and is the subject of this book.

```rust
# extern crate cucumber;
# extern crate tokio;
#
# use std::time::Duration;
#
# use cucumber::{World as _, given, then, when};
# use tokio::time::sleep;
#
# #[derive(cucumber::World, Debug, Default)]
# struct World {
#     user: Option<String>,
#     capacity: usize,
# }
#
#[given(expr = "{word} is hungry")] // Cucumber Expression
async fn someone_is_hungry(w: &mut World, user: String) {
    sleep(Duration::from_secs(2)).await;
    
    w.user = Some(user);
}

#[when(regex = r"^(?:he|she|they) eats? (\d+) cucumbers?$")]
async fn eat_cucumbers(w: &mut World, count: usize) {
    sleep(Duration::from_secs(2)).await;

    w.capacity += count;
    
    assert!(w.capacity < 4, "{} exploded!", w.user.as_ref().unwrap());
}

#[then("she is full")]
async fn is_full(w: &mut World) {
    sleep(Duration::from_secs(2)).await;

    assert_eq!(w.capacity, 3, "{} isn't full!", w.user.as_ref().unwrap());
}
#
# #[tokio::main]
# async fn main() {
#     World::run("tests/features/readme").await;
# }
```
![record](rec/readme.gif)

Since the goal is the testing of externally identifiable behavior of some feature, it would be a misnomer to use [Cucumber] to test specific private aspects or isolated modules. [Cucumber] tests are more likely to take the form of integration, functional or E2E testing.




[`cucumber`]: https://docs.rs/cucumber

[BDD]: https://en.wikipedia.org/wiki/Behavior-driven_development
[Cucumber]: https://cucumber.io
[Gherkin]: https://cucumber.io/docs/gherkin/reference

[1]: https://cucumber.io/docs/gherkin/languages
