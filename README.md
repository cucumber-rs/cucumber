# cucumber-rust

An implementation of the Cucumber testing framework for Rust. Fully native, no external test runners or dependencies.

<p align="center">
    <img src="https://rawcdn.githack.com/bbqsrc/cucumber-rust/aa0c7efe20298d77f0acd3442946290b07026653/example.svg">
</p>

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
use cucumber::{Cucumber, World};
use async_trait::async_trait;

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String
}

#[async_trait(?Send)]
impl World for MyWorld {
    async fn new() -> Self {
        // This function is called every time a new scenario is started
        MyWorld { 
            foo: "a default string".to_string()
        }
    }
}

mod example_steps {
    use cucumber::Steps;
    use futures::future::FutureExt;
    use std::rc::Rc;

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given(
                "a thing",
                Rc::new(|mut world, _step| {
                    async move {
                        world.foo = "elho".into();
                        world
                    }
                    .catch_unwind()
                    .boxed_local()
                }),
            )
            .when_regex(
                "something goes (.*)",
                Rc::new(|world, _matches, _step| async move { world }.catch_unwind().boxed_local()),
            )
            .given_sync(
                "I am trying out Cucumber",
                |mut world: crate::MyWorld, _step| {
                    world.foo = "Some string".to_string();
                    world
                },
            )
            .when_sync("I consider what I am doing", |mut world, _step| {
                let new_string = format!("{}.", &world.foo);
                world.foo = new_string;
                world
            })
            .then_sync("I am interested in ATDD", |world, _step| {
                assert_eq!(world.foo, "Some string.");
                world
            })
            .then_regex_sync(
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

    let runner = Cucumber::<MyWorld>::new()
        .features(&["./features"])
        .steps(example_steps::steps());

    // You may choose any executor you like (Tokio, async-std, etc)
    // You may even have an async main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(runner.run());
}
```

The `cucumber!` creates the `main` function to be run.

The `steps!` macro generates a function named `steps` with all the declared steps in the module
it is defined in. Ordinarily you would create something like a `steps/` directory to hold your 
steps modules instead of inline like the given example.

The full gamut of Cucumber's Gherkin language is implemented by the 
[gherkin-rust](https://github.com/bbqsrc/gherkin-rust) project. Most features of the Gherkin 
language are parsed already and accessible via the relevant structs.

### License

This project is licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
