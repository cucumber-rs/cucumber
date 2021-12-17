Asserting
=========

There are two ways of doing [assertion]s in a [step] matching function: 
- throwing a panic;
- returning an error.




## Panic

Throwing a panic in a [step] matching function makes the appropriate [step] failed:
```rust
# use std::convert::Infallible;
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
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
# #[given(regex = r"^a (hungry|satiated) cat$")]
# fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     match state.as_str() {
#         "hungry" =>  world.cat.hungry = true,
#         "satiated" =>  world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# fn feed_cat(world: &mut AnimalWorld) {
#     world.cat.feed();
# }
#
#[then("the cat is not hungry")]
fn cat_is_fed(_: &mut AnimalWorld) {
    panic!("Cats are always hungry!")
}
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/writing/asserting.feature").await;
# }
```
![record](../rec/writing_asserting_panic.gif)

> __NOTE__: Failed [step] prints its location in a `.feature` file, the captured [assertion] message, and state of the `World` at the moment of failure.





## `FromStr` arguments

For matching a captured value we are not restricted to use only `String`. In fact, any type implementing a [`FromStr`] trait can be used as a [step] function argument (including primitive types).

```rust
# use std::{convert::Infallible, str::FromStr};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
#
# #[derive(Debug)]
# struct Cat {
#     pub hungry: State,
# }
#
# impl Cat {
#     fn feed(&mut self) {
#         self.hungry = State::Satiated;
#     }
# }
#
#[derive(Debug)]
enum State {
    Hungry,
    Satiated,
}

impl FromStr for State {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "hungry" => Self::Hungry,
            "satiated" => Self::Satiated,
            invalid => return Err(format!("Invalid `State`: {}", invalid)),
        })
    }
}
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
#             cat: Cat {
#                 hungry: State::Satiated,
#             },
#         })
#     }
# }

#[given(regex = r"^a (hungry|satiated) cat$")]
fn hungry_cat(world: &mut AnimalWorld, state: State) {
    world.cat.hungry = state;
}

#[when(regex = r"^I feed the cat (\d+) times?$")]
fn feed_cat(world: &mut AnimalWorld, times: u8) {
    for _ in 0..times {
        world.cat.feed();
    }
}
# 
# #[then("the cat is not hungry")]
# fn cat_is_fed(world: &mut AnimalWorld) {
#     assert!(matches!(world.cat.hungry, State::Satiated));
# }
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/writing/capturing.feature").await;
# }
```
![record](../rec/writing_capturing_both.gif)




## Cucumber Expressions

Alternatively, a [Cucumber Expression][expr] may be used to capture values. This is possible with `expr =` attribute modifier and [parameters] usage:
```rust
# use std::{convert::Infallible, str::FromStr};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
#
# #[derive(Debug)]
# struct Cat {
#     pub hungry: State,
# }
#
# impl Cat {
#     fn feed(&mut self) {
#         self.hungry = State::Satiated;
#     }
# }
#
# #[derive(Debug)]
# enum State {
#     Hungry,
#     Satiated,
# }
#
# impl FromStr for State {
#     type Err = String;
#
#     fn from_str(s: &str) -> Result<Self, Self::Err> {
#         Ok(match s {
#             "hungry" => Self::Hungry,
#             "satiated" => Self::Satiated,
#             invalid => return Err(format!("Invalid `State`: {}", invalid)),
#         })
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
#             cat: Cat {
#                 hungry: State::Satiated,
#             },
#         })
#     }
# }
#
#[given(expr = "a {word} cat")]
fn hungry_cat(world: &mut AnimalWorld, state: State) {
    world.cat.hungry = state;
}

#[when(expr = "I feed the cat {int} time(s)")]
fn feed_cat(world: &mut AnimalWorld, times: u8) {
    for _ in 0..times {
        world.cat.feed();
    }
}
# 
# #[then("the cat is not hungry")]
# fn cat_is_fed(world: &mut AnimalWorld) {
#     assert!(matches!(world.cat.hungry, State::Satiated));
# }
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/writing/capturing.feature").await;
# }
```

[Cucumber Expressions][expr] are less powerful in terms of parsing and capturing values, but are much more readable than [regular expressions][regex], so it's worth to prefer using them for simple matching.

![record](../rec/writing_capturing_both.gif)

> __NOTE__: Captured [parameters] are __bold__ to indicate which part of a [step] is actually captured.


### Custom [parameters]

Another useful advantage of using [Cucumber Expressions][expr] is an ability to declare and reuse  [custom parameters] in addition to [default ones][parameters].

```rust
# use std::{convert::Infallible, str::FromStr};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
use cucumber::Parameter;

# #[derive(Debug)]
# struct Cat {
#     pub hungry: State,
# }
#
# impl Cat {
#     fn feed(&mut self) {
#         self.hungry = State::Satiated;
#     }
# }
#
#[derive(Debug, Parameter)]
// NOTE: `name` is optional, by default the lowercased type name is implied.
#[param(name = "hungriness", regex = "hungry|satiated")]
enum State {
    Hungry,
    Satiated,
}

// NOTE: `Parameter` requires `FromStr` being implemented.
impl FromStr for State {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "hungry" => Self::Hungry,
            "satiated" => Self::Satiated,
            invalid => return Err(format!("Invalid `State`: {}", invalid)),
        })
    }
}
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
#             cat: Cat {
#                 hungry: State::Satiated,
#             },
#         })
#     }
# }

#[given(expr = "a {hungriness} cat")]
fn hungry_cat(world: &mut AnimalWorld, state: State) {
    world.cat.hungry = state;
}
#
# #[when(expr = "I feed the cat {int} time(s)")]
# fn feed_cat(world: &mut AnimalWorld, times: u8) {
#     for _ in 0..times {
#         world.cat.feed();
#     }
# }
# 
# #[then("the cat is not hungry")]
# fn cat_is_fed(world: &mut AnimalWorld) {
#     assert!(matches!(world.cat.hungry, State::Satiated));
# }
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/writing/capturing.feature").await;
# }
```

> __NOTE__: Using [custom parameters] allows declaring and reusing complicated and precise matches without a need to repeat them in different [step] matching functions.

![record](../rec/writing_capturing_both.gif)




[`FromStr`]: https://doc.rust-lang.org/stable/std/str/trait.FromStr.html
[custom parameters]: https://github.com/cucumber/cucumber-expressions#custom-parameter-types
[expr]: https://cucumber.github.io/cucumber-expressions
[parameters]: https://github.com/cucumber/cucumber-expressions#parameter-types
[regex]: https://en.wikipedia.org/wiki/Regular_expression


[assertion]: https://en.wikipedia.org/wiki/Assertion_(software_development)
[step]: https://cucumber.io/docs/gherkin/reference#steps
