[![Build Status](https://travis-ci.org/bbqsrc/cucumber-rust.svg?branch=master)](https://travis-ci.org/bbqsrc/cucumber-rust)

# cucumber-rust

An implementation of the Cucumber testing framework for Rust. Fully native, no external test runners or dependencies.

<p align="center">
    <img src="https://cdn.rawgit.com/bbqsrc/cucumber-rust/develop/example.svg">
</p>

### Usage

Create a directory called `tests/` in your project root and create a test target of your choice. In this example we will name it `cucumber.rs`.

Add this to your `Cargo.toml`:

```toml
[[test]]
name = "cucumber"
harness = false # Allows Cucumber to print output instead of libtest

[dev-dependencies]
cucumber = { package = "cucumber_rust", version = "^0.6.0" } 
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
use cucumber::{cucumber, steps, before, after};

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String
}

impl cucumber::World for MyWorld {}
impl std::default::Default for MyWorld {
    fn default() -> MyWorld {
        // This function is called every time a new scenario is started
        MyWorld { 
            foo: "a default string".to_string()
        }
    }
}

mod example_steps {
    use cucumber::steps;
    
    // Any type that implements cucumber::World + Default can be the world
    steps!(::MyWorld => {
        given "I am trying out Cucumber" |world, step| {
            world.foo = "Some string".to_string();
            // Set up your context in given steps
        };

        when "I consider what I am doing" |world, step| {
            // Take actions
            let new_string = format!("{}.", &world.foo);
            world.foo = new_string;
        };

        then "I am interested in ATDD" |world, step| {
            // Check that the outcomes to be observed have occurred
            assert_eq!(world.foo, "Some string.");
        };

        then regex r"^we can (.*) rules with regex$" |world, matches, step| {
            // And access them as an array
            assert_eq!(matches[1], "implement");
        };

        then regex r"^we can also match (\d+) (.+) types$" (usize, String) |world, num, word, step| {
            // `num` will be of type usize, `word` of type String
            assert_eq!(num, 42);
            assert_eq!(word, "olika");
        };

        then "we can use data tables to provide more parameters" |world, step| {
            let table = step.table().unwrap().clone();

            assert_eq!(table.header, vec!["key", "value"]);

            let expected_keys = table.rows.iter().map(|row| row[0].to_owned()).collect::<Vec<_>>();
            let expected_values = table.rows.iter().map(|row| row[1].to_owned()).collect::<Vec<_>>();

            assert_eq!(expected_keys, vec!["a", "b"]);
            assert_eq!(expected_values, vec!["fizz", "buzz"]);
        };
    });
}

// Declares a before handler function named `a_before_fn`
before!(a_before_fn => |scenario| {

});

// Declares an after handler function named `an_after_fn`
after!(an_after_fn => |scenario| {

});

// A setup function to be called before everything else
fn setup() {
    
}

cucumber! {
    features: "./features", // Path to our feature files
    world: ::MyWorld, // The world needs to be the same for steps and the main cucumber call
    steps: &[
        example_steps::steps // the `steps!` macro creates a `steps` function in a module
    ],
    setup: setup, // Optional; called once before everything
    before: &[
        a_before_fn // Optional; called before each scenario
    ], 
    after: &[
        an_after_fn // Optional; called after each scenario
    ] 
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
