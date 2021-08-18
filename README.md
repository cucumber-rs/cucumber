# cucumber-rust

[![Documentation](https://docs.rs/cucumber_rust/badge.svg)](https://docs.rs/cucumber_rust)
[![Actions Status](https://github.com/bbqsrc/cucumber-rust/workflows/CI/badge.svg)](https://github.com/bbqsrc/cucumber-rust/actions)
[![Unsafe Forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

An implementation of the Cucumber testing framework for Rust. Fully native, no external test runners or dependencies.

- [Changelog](CHANGELOG.md)

## Usage

Describe testing scenarios in `.feature` files.

```gherkin
Feature: eating too much cucumbers may not be good for you
    
  Scenario: Eating a few isn't a problem
    Given Alice is hungry
    When she eats 3 cucumbers
    Then she is full
```

Implement `World` trait and describe steps.

 ```rust
use std::{convert::Infallible, time::Duration};

use async_trait::async_trait;
use cucumber_rust::{self as cucumber, given, then, when, WorldInit};
use tokio::time::sleep;

#[derive(Debug, WorldInit)]
struct World {
    user: Option<String>,
    capacity: usize,
}

#[async_trait(? Send)]
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

#[when(regex = r"^(?:he|she|they) eat (\d+) cucumbers?$")]
async fn eat_cucumbers(w: &mut World, count: usize) {
    sleep(Duration::from_secs(2)).await;

    w.capacity += count;

    if w.capacity > 3 {
        panic!("{} exploded!", w.user.as_ref().unwrap());
    }
}

#[then(regex = r"^(?:he|she|they) (?:is|are) full$")]
async fn is_full(w: &mut World) {
    sleep(Duration::from_secs(2)).await;

    assert_eq!(
        w.capacity, 3, 
        "{} isn't full!", 
        w.user.as_ref().unwrap(),
    );
}

#[tokio::main]
async fn main() {
    World::run("tests/features/example").await;
}
```

Output

[![asciicast](https://asciinema.org/a/6AEi2r6qdl7c4CnKzjhcS673u.svg)](https://asciinema.org/a/6AEi2r6qdl7c4CnKzjhcS673u)

### Supporting crates

The full gamut of Cucumber's Gherkin language is implemented by the 
[gherkin-rust](https://github.com/bbqsrc/gherkin-rust) project. Most features of the Gherkin 
language are parsed already and accessible via the relevant structs.

### License

This project is licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
