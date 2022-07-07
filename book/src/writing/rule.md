`Rule` keyword
==============

The purpose of the [`Rule` keyword][rule] is to represent a business rule that should be implemented. It provides additional information for a [feature]. A [`Rule`][rule] is used to group together several [scenario]s belonging to the business rule. A [`Rule`][rule] should contain one or more [scenario]s illustrating the particular rule.

No additional work is required on the implementation side to support [`Rule`s][rule].

```gherkin
Feature: Animal feature
    
  Rule: Hungry cat becomes satiated
      
    Scenario: If we feed a hungry cat it will no longer be hungry
      Given a hungry cat
      When I feed the cat
      Then the cat is not hungry
    
  Rule: Satiated cat remains the same
      
    Scenario: If we feed a satiated cat it will not become hungry
      Given a satiated cat
      When I feed the cat
      Then the cat is not hungry
```
```rust
# use std::time::Duration;
#
# use cucumber::{given, then, when, World, WorldInit as _};
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Cat {
#     pub hungry: bool,
# }
#
# impl Cat {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, Default, World)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
#
#[given(regex = r"^a (hungry|satiated) cat$")]
async fn hungry_cat(world: &mut AnimalWorld, state: String) {
    sleep(Duration::from_secs(2)).await;

    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when("I feed the cat")]
async fn feed_cat(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    world.cat.feed();
}

#[then("the cat is not hungry")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("tests/features/book/writing/rule.feature").await;
# }
```
![record](../rec/writing_rule.gif)




[feature]: https://cucumber.io/docs/gherkin/reference#feature
[rule]: https://cucumber.io/docs/gherkin/reference#rule
[scenario]: https://cucumber.io/docs/gherkin/reference#example
