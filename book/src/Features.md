# Features

## Rule

The purpose of the `Rule` keyword is to represent one business rule that should be implemented. It provides additional information for a feature. A `Rule` is used to group together several scenarios that belong to this business rule. A `Rule` should contain one or more scenarios that illustrate the particular rule.

You don't need additional work on the implementation side to support `Rule`s. Let's take final example from [Getting Started](Getting_Started.md) chapter and change `.feature` file to:

```gherkin
Feature: Animal feature
    
  Rule: hungry cat  
      
    Scenario: If we feed a hungry cat it will no longer be hungry
      Given a hungry cat
      When I feed the cat
      Then the cat is not hungry
    
  Rule: satiated cat
      
    Scenario: If we feed a satiated cat it will not become hungry
      Given a satiated cat
      When I feed the cat
      Then the cat is not hungry
```

<script id="asciicast-E9QvyDjEYBc5MKJpADxyiD7Ax" src="https://asciinema.org/a/E9QvyDjEYBc5MKJpADxyiD7Ax.js" async data-autoplay="true" data-rows="18"></script>

## Background

Occasionally you’ll find yourself repeating the same `Given` steps in all the scenarios in a Feature.

Since it is repeated in every scenario, this is an indication that those steps are not essential to describe the scenarios; they are _incidental details_. You can literally move such `Given` steps to the background, by grouping them under a `Background` section.

```gherkin
Feature: Animal feature
    
  Background: 
    Given a hungry cat
    
  Rule: hungry cat  
      
    Scenario: If we feed a hungry cat it will no longer be hungry
      When I feed the cat
      Then the cat is not hungry
    
  Rule: satiated cat
      
    Background:
      When I feed the cat

    Scenario: If we feed a satiated cat it will not become hungry
      When I feed the cat
      Then the cat is not hungry
```

<script id="asciicast-PQ5bfRNBqBehFpIddM4Y0qCAd" src="https://asciinema.org/a/PQ5bfRNBqBehFpIddM4Y0qCAd.js" async data-autoplay="true" data-rows="18"></script>

`Background` `Step`s indicated by `>` by default.

In case `Background` is declared outside any `Rule`, it will be run firstly on any `Scenario`. Otherwise, if `Background` is declared inside `Rule`, it will be run only for `Scenario`s inside this `Rule` and only after top-level `Background` statements, if any.

### Tips for using Background

 - Don’t use `Background` to set up complicated states, unless that state is actually something the client needs to know.
 - Keep your `Background` section short.
 - Make your `Background` section vivid, use colourful names, and try to tell a story.
 - Keep your `Scenario`s short, and don’t have too many.

Clearly, example provided above doesn't need `Background` and done for demonstration purposes only.

## Scenario Outline

The `Scenario Outline` keyword can be used to run the same `Scenario` multiple times, with different combinations of values:

```gherkin
Feature: Animal feature

  Scenario Outline: If we feed a hungry animal it will no longer be hungry
    Given a hungry <animal>
    When I feed the <animal>
    Then the <animal> is not hungry

  Examples: 
    | animal |
    | cat    |
    | dog    |
    | 🦀     |
```

And leverage regex support to match `Step`s:

```rust
# use std::{convert::Infallible, time::Duration};
# 
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
# 
# #[derive(Debug)]
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
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
# 
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
# 
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Cat { hungry: false },
#         })
#     }
# }
# 
#[given(regex = r"^a (hungry|satiated) (\S+)$")]
async fn hungry_cat(world: &mut AnimalWorld, state: String) {
    sleep(Duration::from_secs(2)).await;

    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when(regex = r"^I feed the (\S+)$")]
async fn feed_cat(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    world.cat.feed();
}

#[then(regex = r"^the (\S+) is not hungry$")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}
# 
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/features/scenario_outline.feature").await;
# }
```

<script id="asciicast-15ZcRGFBUXubvcle34ZOLiLtO" src="https://asciinema.org/a/15ZcRGFBUXubvcle34ZOLiLtO.js" async data-autoplay="true" data-rows="18"></script>

### Combining Regex and FromStr

At parsing stage, `<templates>` are replaced by value from cells. That means you can parse table calls into any type, that implements [`FromStr`](https://doc.rust-lang.org/stable/std/str/trait.FromStr.html)

```gherkin
Feature: Animal feature

  Scenario Outline: If we feed a hungry animal it will no longer be hungry
    Given a hungry <animal>
    When I feed the <animal> <n> times
    Then the <animal> is not hungry

  Examples: 
    | animal | n |
    | cat    | 2 |
    | dog    | 3 |
    | 🦀     | 4 |
```

```rust
# use std::{convert::Infallible, str::FromStr, time::Duration};
# 
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
# 
# #[derive(Debug)]
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
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
# 
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
# 
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Cat { hungry: false },
#         })
#     }
# }
# 
enum State {
    Hungry,
    Satiated,
}

impl FromStr for State {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hungry" => Ok(State::Hungry),
            "satiated" => Ok(State::Satiated),
            _ => Err("expected hungry or satiated"),
        }
    }
}

#[given(regex = r"^a (\S+) (\S+)$")]
async fn hungry_cat(world: &mut AnimalWorld, state: State) {
    sleep(Duration::from_secs(2)).await;

    match state {
        State::Hungry => world.cat.hungry = true,
        State::Satiated => world.cat.hungry = false,
    }
}

#[when(regex = r"^I feed the (?:\S+)(?: (\d+) times?)?$")]
async fn feed_cat(world: &mut AnimalWorld, times: usize) {
    sleep(Duration::from_secs(2)).await;

    for _ in 0..times {
        world.cat.feed();
    }
}

#[then(regex = r"^the (\S+) is not hungry$")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}
# 
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/features/scenario_outline_fromstr.feature").await;
# }
```

<script id="asciicast-joMErjGUVegtXPJgL8fc5x6pt" src="https://asciinema.org/a/joMErjGUVegtXPJgL8fc5x6pt.js" async data-autoplay="true" data-rows="18"></script>

## Spoken Languages

The language you choose for `Gherkin` should be the same language your users and domain experts use when they talk about the domain. Translating between two languages should be avoided.

This is why Gherkin has been translated to over [70 languages](https://cucumber.io/docs/gherkin/languages/).

A `# language:` header on the first line of a feature file tells Cucumber what spoken language to use - for example `# language: fr` for French. If you omit this header, Cucumber will default to English (`en`).

```gherkin
# language: no
    
Egenskap: Animal feature
    
  Eksempel: If we feed a hungry cat it will no longer be hungry
    Gitt a hungry cat
    Når I feed the cat
    Så the cat is not hungry
```

<script id="asciicast-sDt8aoo9ZVPZRgiTuy8pSNro2" src="https://asciinema.org/a/sDt8aoo9ZVPZRgiTuy8pSNro2.js" async data-autoplay="true" data-rows="18"></script>

In case most of your `.feature` files aren't written in English and you want to avoid endless `# language:` comments, use [`Cucumber::language()`](https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.language) method to override default language.
