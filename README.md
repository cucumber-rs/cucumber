Cucumber testing framework for Rust
===================================

[![Documentation](https://docs.rs/cucumber/badge.svg)](https://docs.rs/cucumber)
[![CI](https://github.com/cucumber-rs/cucumber/workflows/CI/badge.svg?branch=master "CI")](https://github.com/cucumber-rs/cucumber/actions?query=workflow%3ACI+branch%3Amaster)
[![Rust 1.56+](https://img.shields.io/badge/rustc-1.56+-lightgray.svg "Rust 1.56+")](https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html)

An implementation of the [Cucumber] testing framework for Rust. Fully native, no external test runners or dependencies.

- Book ([current][1] | [edge][2])
- [Changelog](https://github.com/cucumber-rs/cucumber/blob/main/CHANGELOG.md)




## Usage

Describe testing scenarios in `.feature` files:
```gherkin
## /tests/features/readme/eating.feature
    
Feature: Eating too much cucumbers may not be good for you
    
  Scenario: Eating a few isn't a problem
    Given Alice is hungry
    When she eats 3 cucumbers
    Then she is full
```

Implement `World` trait and describe steps:
 ```rust
//! tests/readme.rs 

use std::{convert::Infallible, time::Duration};

use async_trait::async_trait;
use cucumber::{given, then, when, WorldInit};
use tokio::time::sleep;

#[derive(Debug, WorldInit)]
struct World {
    user: Option<String>,
    capacity: usize,
}

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self { user: None, capacity: 0 })
    }
}

#[given(regex = r"^(\S+) is hungry$")]
async fn someone_is_hungry(w: &mut World, user: String) {
    sleep(Duration::from_secs(2)).await;
    
    w.user = Some(user);
}

#[when(regex = r"^(?:he|she|they) eats? (\d+) cucumbers?$")]
async fn eat_cucumbers(w: &mut World, count: usize) {
    sleep(Duration::from_secs(2)).await;

    w.capacity += count;
    
    assert!(w.capacity < 4, "{} exploded!", w.user.as_ref().unwrap());
}

#[then(regex = r"^(?:he|she|they) (?:is|are) full$")]
async fn is_full(w: &mut World) {
    sleep(Duration::from_secs(2)).await;

    assert_eq!(w.capacity, 3, "{} isn't full!", w.user.as_ref().unwrap());
}

#[tokio::main]
async fn main() {
    World::run("tests/features/readme").await;
}
```

Add test to `Cargo.toml`:
```toml
[[test]]
name = "readme"
harness = false  # allows Cucumber to print output instead of libtest
```

[![asciicast](https://asciinema.org/a/6wN3uv8p98SgVznPUh9h50bFo.svg)](https://asciinema.org/a/6wN3uv8p98SgVznPUh9h50bFo)

For more examples check out the Book ([current][1] | [edge][2]).




## Cargo features

- `macros` (default): Enables step attributes and auto-wiring.
- `timestamps`: Enables timestamps collecting for all [Cucumber] events.
- `output-json` (implies `timestamps`): Enables support for outputting in [Cucumber JSON format].
- `output-junit` (implies `timestamps`): Enables support for outputting [JUnit XML report].




## Supporting crates

The full gamut of Cucumber's [Gherkin] language is implemented by the [`gherkin-rust`](https://github.com/bbqsrc/gherkin-rust) crate. Most features of the [Gherkin] language are parsed already and accessible via the relevant structs.




## Known issues

- `Scenario Outline` is treated the same as `Outline` or `Example` in the parser ([gherkin/#19](https://github.com/bbqsrc/gherkin-rust/issues/19)).




## License

This project is licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/cucumber-rs/cucumber/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT](https://github.com/cucumber-rs/cucumber/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.




[Cucumber]: https://cucumber.io
[Cucumber JSON format]: https://github.com/cucumber/cucumber-json-schema
[Gherkin]: https://cucumber.io/docs/gherkin/reference
[JUnit XML report]: https://llg.cubic.org/docs/junit

[1]: https://cucumber-rs.github.io/cucumber/current
[2]: https://cucumber-rs.github.io/cucumber/main
