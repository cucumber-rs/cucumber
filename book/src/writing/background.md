`Background` keyword
====================

Occasionally, we may find ourselves repeating the same [`Given`] [step]s in all the [scenario]s of a [feature].

Since it's repeated in each [scenario], this is an indication that those [step]s are not quite _essential_ to describe the [scenario]s, but rather are _incidental details_. So, we can move such [`Given`] [step]s to background, by grouping them under a [`Background`] section.

[`Background`] allows you to add some context to the [scenario]s following it. It can contain one or more [step]s, which are run before each [scenario] (but after any [`Before` hooks][hook]).

```gherkin
Feature: Animal feature
    
  Background: 
    Given a hungry cat
    
  Rule: Hungry cat becomes satiated
      
    Scenario: If we feed a hungry cat it will no longer be hungry
      When I feed the cat
      Then the cat is not hungry
    
  Rule: Satiated cat remains the same
      
    Background:
      When I feed the cat

    Scenario: If we feed a satiated cat it will not become hungry
      When I feed the cat
      Then the cat is not hungry
```
```rust
# extern crate cucumber;
# extern crate tokio;
#
# use std::time::Duration;
#
# use cucumber::{World, given, then, when};
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
#     AnimalWorld::run("tests/features/book/writing/background.feature").await;
# }
```
![record](../rec/writing_background.gif)

> __NOTE__: [`Background`] [step]s indicated by `>` mark in the output.

> __NOTE__: In case [`Background`] is declared outside any [`Rule`], it will be run on any [scenario]. Otherwise, if [`Background`] is declared inside a [`Rule`], it will be run only for [scenario]s belonging to it, and only after top-level [`Background`] [step]s (if any).




## Best practices

- Don’t use [`Background`] to set up complicated states, unless that state is actually something the client needs to know.
- Keep your [`Background`] section short.
- Make your [`Background`] section vivid, use colorful names, and try to tell a story.
- Keep your [`Scenario`s][scenario] short, and don’t have too many.

Clearly, example provided above doesn't need [`Background`] and was made for demonstration purposes only.




[`Background`]: https://cucumber.io/docs/gherkin/reference#background
[`Given`]: https://cucumber.io/docs/gherkin/reference#given
[`Rule`]: https://cucumber.io/docs/gherkin/reference#rule
[feature]: https://cucumber.io/docs/gherkin/reference#feature
[hook]: https://cucumber.io/docs/cucumber/api#before
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[step]: https://cucumber.io/docs/gherkin/reference#steps
