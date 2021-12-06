`Scenario Outline` keyword
==========================

The [`Scenario Outline`] keyword can be used to run the same [scenario] multiple times, with different combinations of values.

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

At parsing stage `<template>`s are replaced by value from cells, so we may get that value in [step] matching functions (if we need though).

> __NOTE__: `<template>`s are replaced even inside [doc strings] and [data tables].

```rust
# use std::{collections::HashMap, convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
#
#[derive(Debug, Default)]
struct Animal {
    pub hungry: bool,
}

impl Animal {
    fn feed(&mut self) {
        self.hungry = false;
    }
}

#[derive(Debug, WorldInit)]
pub struct AnimalWorld {
    animals: HashMap<String, Animal>,
}

#[async_trait(?Send)]
impl World for AnimalWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            animals: HashMap::new(),
        })
    }
}

#[given(regex = r"^a (hungry|satiated) (\S+)$")]
async fn hungry_animal(world: &mut AnimalWorld, state: String, which: String) {
    sleep(Duration::from_secs(2)).await;

    world.animals.entry(which).or_insert(Animal::default()).hungry =
        match state.as_str() {
            "hungry" => true,
            "satiated" => false,
            _ => unreachable!(),
        };
}

#[when(expr = "I feed the {word} {int} time(s)")]
async fn feed_animal(world: &mut AnimalWorld, which: String, times: usize) {
    sleep(Duration::from_secs(2)).await;

    for _ in 0..times {
        world.animals.get_mut(&which).map(Animal::feed);
    }
}

#[then(expr = "the {word} is not hungry")]
async fn animal_is_fed(world: &mut AnimalWorld, which: String) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.animals.get(&which).map_or(true, |a| a.hungry));
}
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/writing/scenario_outline.feature").await;
# }
```

> __NOTE__: [`Scenario Outline`] runs the whole [scenario] for each table row separately, unlike [data tables], which run the whole table inside a single [step].

![record](../rec/writing_scenario_outline.gif)




[`Scenario Outline`]: https://cucumber.io/docs/gherkin/reference#scenario-outline
[data tables]: data_tables.md
[doc strings]: doc_strings.md
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[step]: https://cucumber.io/docs/gherkin/reference#steps
