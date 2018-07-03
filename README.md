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
cucumber = "^0.3.2"
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
extern crate cucumber_rust;

pub struct World {
    // You can use this struct for mutable context in scenarios.
}

impl cucumber_rust::World for MyWorld {}
impl std::default::Default for MyWorld {
    fn default() -> MyWorld {
        // This function is called every time a new scenario is started
        MyWorld { }
    }
}

mod example_steps {
    steps! {
        world: ::MyWorld; // Any type that implements Default can be the world

        given "I am trying out Cucumber" |world, step| {
            // Set up your context in given steps
        };

        when "I consider what I am doing" |world, step| {
            // Take actions
        };

        then "I am interested in ATDD" |world, step| {
            // Check that the outcomes to be observed have occurred
        };

        then regex r"^we can (.*) rules with regex$" |world, matches, step| {
            // And access them as an array
            assert_eq!(matches[1], "implement");
        };
    }
}

cucumber! {
    features: "./features"; // Path to our feature files
    world: ::MyWorld; // The world needs to be the same for steps and the main cucumber call
    steps: &[
        example_steps::steps // the `steps!` macro creates a `steps` function in a module
    ],
    before: || {
      // Called once before everything; optional.
    }
}
```

The `cucumber!` creates the `main` function to be run.

The `steps!` macro generates a function named `steps` with all the declared steps in the module
it is defined in. Ordinarily you would create something like a `steps/` directory to hold your 
steps modules instead of inline like the given example.

The full gamut of Cucumber's Gherkin language is implemented by the 
[gherkin-rust](https://github.com/bbqsrc/gherkin-rust) project. Features such
as data tables and docstrings will be progressively implemented prior to
v1.0.0.

### License

This project is licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
