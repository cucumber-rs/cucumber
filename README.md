# cucumber-rust

[![Documentation](https://docs.rs/cucumber_rust/badge.svg)](https://docs.rs/cucumber_rust)
[![Actions Status](https://github.com/bbqsrc/cucumber-rust/workflows/CI/badge.svg)](https://github.com/bbqsrc/cucumber-rust/actions)

An implementation of the Cucumber testing framework for Rust. Fully native, no external test runners or dependencies.

**The 0.6.x macro has been removed. If you need to use this macro, please continue to use the 0.6 version.** 

- [Changelog](CHANGELOG.md)
- [Cucumber in Rust 0.7 – Beginner’s Tutorial](https://www.florianreinhard.de/2020-10-05/cucumber-07-in-rust-beginners-tutorial/) by Florian Reinhard.

### Usage

Create a directory called `tests/` in your project root and create a test target of your choice. In this example we will name it `cucumber.rs`.

Add this to your `Cargo.toml`:

```toml
[[test]]
name = "cucumber"
harness = false # Allows Cucumber to print output instead of libtest

[dev-dependencies]
cucumber = { package = "cucumber_rust", version = "^0.7.0" } 
```

Create a directory called `features/` and put a feature file in it named something like `example.feature`. It might look like:

```gherkin
Feature: Example feature

  Scenario: An example scenario
    Given I am trying out Cucumber
    When I consider what I am doing
    Then I am interested in ATDD
    And we can implement rules with regex

```

And here's an example of implementing those steps using our `tests/cucumber.rs` file:

```rust
use async_trait::async_trait;
use std::{convert::Infallible, cell::RefCell};

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
    bar: usize,
    some_value: RefCell<u8>,
}

impl MyWorld {
    async fn test_async_fn(&mut self) {
        *self.some_value.borrow_mut() = 123u8;
        self.bar = 123;
    }
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            foo: "wat".into(),
            bar: 0,
            some_value: RefCell::new(0),
        })
    }
}

mod example_steps {
    use cucumber::{Steps, t};

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given_async(
                "a thing",
                t!(|mut world, _step| {
                    world.foo = "elho".into();
                    world.test_async_fn().await;
                    world
                })
            )
            .when_regex_async(
                "something goes (.*)",
                t!(|world, _matches, _step| world),
            )
            .given(
                "I am trying out Cucumber",
                |mut world: crate::MyWorld, _step| {
                    world.foo = "Some string".to_string();
                    world
                },
            )
            .when("I consider what I am doing", |mut world, _step| {
                let new_string = format!("{}.", &world.foo);
                world.foo = new_string;
                world
            })
            .then("I am interested in ATDD", |world, _step| {
                assert_eq!(world.foo, "Some string.");
                world
            })
            .then_regex(
                r"^we can (.*) rules with regex$",
                |world, matches, _step| {
                    // And access them as an array
                    assert_eq!(matches[1], "implement");
                    world
                },
            );

        builder
    }
}

fn main() {
    // Do any setup you need to do before running the Cucumber runner.
    // e.g. setup_some_db_thing()?;

    let runner = cucumber::Cucumber::<MyWorld>::new()
        .features(&["./features"])
        .steps(example_steps::steps());

    // You may choose any executor you like (Tokio, async-std, etc)
    // You may even have an async main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(runner.run());
}
```

You can then run your Cucumber tests by running this command:

```
cargo test --test cucumber
```

#### Auto-wiring via macros

By enabling `macros` feature in `Cargo.toml`:
```toml
[[test]]
name = "cucumber"
harness = false # Allows Cucumber to print output instead of libtest

[dev-dependencies]
cucumber_rust = { version = "^0.7.0", features = ["macros"] } 
```

You could leverage some conveniences in organizing your tests code:
```rust
use std::{cell::RefCell, convert::Infallible};

use async_trait::async_trait;
use cucumber_rust::{given, then, when, World, WorldInit};

#[derive(WorldInit)]
pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
    bar: usize,
    some_value: RefCell<u8>,
}

impl MyWorld {
    async fn test_async_fn(&mut self) {
        *self.some_value.borrow_mut() = 123u8;
        self.bar = 123;
    }
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            foo: "wat".into(),
            bar: 0,
            some_value: RefCell::new(0),
        })
    }
}

#[given("a thing")]
async fn a_thing(world: &mut MyWorld) {
    world.foo = "elho".into();
    world.test_async_fn().await;
}

#[when(regex = "something goes (.*)")]
async fn something_goes(_: &mut MyWorld, _wrong: String) {}

#[given("I am trying out Cucumber")]
fn i_am_trying_out(world: &mut MyWorld) {
    world.foo = "Some string".to_string();
}

#[when("I consider what I am doing")]
fn i_consider(world: &mut MyWorld) {
    let new_string = format!("{}.", &world.foo);
    world.foo = new_string;
}

#[then("I am interested in ATDD")]
fn i_am_interested(world: &mut MyWorld) {
    assert_eq!(world.foo, "Some string.");
}

#[then(regex = r"^we can (.*) rules with regex$")]
fn we_can_regex(_: &mut MyWorld, action: String) {
    // `action` can be anything implementing `FromStr`.
    assert_eq!(action, "implement");
}

fn main() {
    let runner = MyWorld::init(&["./features"]);
    futures::executor::block_on(runner.run_and_exit());
}
```

### Supporting crates

The full gamut of Cucumber's Gherkin language is implemented by the 
[gherkin-rust](https://github.com/bbqsrc/gherkin-rust) project. Most features of the Gherkin 
language are parsed already and accessible via the relevant structs.

### License

This project is licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
