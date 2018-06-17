# cucumber-rust

An implementation of the Cucumber testing framework for Rust. Fully native, no external test runners or dependencies.

### Usage

Create a directory called `tests/` in your project root and create a test target of your choice. In this example we will name it `cucumber.rs`.

Add this to your `Cargo.toml`:

```toml
[[test]]
name = "cucumber"
harness = false # Allows Cucumber to print output instead of libtest

[dev-dependencies]
cucumber = "^0.1"
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
#[macro_use]
extern crate cucumber;

pub struct World {
    // You can use this struct for mutable context in scenarios.
}

impl std::default::Default for World {
    fn default() -> World {
        // This function is called every time a new scenario is started
        World { }
    }
}

cucumber! {
    features: "./features"; // Path to our feature files
    world: World; // Any type that implements Default can be the world

    given "I am trying out Cucumber" |world| {
        // Set up your context in given steps
    };

    when "I consider what I am doing" |world| {
        // Take actions
    };

    then "I am interested in ATDD" |world| {
        // Check that the outcomes to be observed have occurred
    };

    then regex r"^we can (.*) rules with regex$" |world, matches| {
        // And access them as an array
        assert!(matches[1] == "implement");
    };
}
```

The full gamut of Cucumber's Gherkin language is implemented by the 
[gherkin-rust](https://github.com/bbqsrc/gherkin-rust) project. Features such
as data tables and docstrings will be progressively implemented prior to
v1.0.0.

### License

This project is licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
