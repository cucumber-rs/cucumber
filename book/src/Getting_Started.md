# Getting Started

Adding Cucumber to your project requires some groundwork. Cucumber tests are ran along with other tests via `cargo test`, but rely on a `.feature` file corresponding to the given test, as well as a set of steps described in code that corresponds to the steps in the feature file.

To start, create a directory called `tests/` in the root of your project and add a file to represent your test target; in this walkthrough we use `example.rs`.

Add this to your Cargo.toml:

```toml
[dev-dependencies]
async-trait = "0.1" # This is currently required to properly initialize the world in cucumber-rust
cucumber_rust = "0.10"
futures = "0.3"

[[test]]
name = "example" # This should be the same as the filename of your test target
harness = false # Allows Cucumber to print output instead of libtest
```

At this point, while it won't do anything, you should be able to successfully run `cargo test --test example` without errors as long as your `example.rs` has at least a `main()` function.

Create a directory called `features/` somewhere in your project, in this walkthrough we use `./tests/features/book` directory. Put a feature file there, such as `animal.feature`. This should contain the Gherkin for a scenario that you want to test. Here's a very simple example:

```gherkin
Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given A hungry cat
    When I feed the cat
    Then The cat is not hungry

```

Here is how we actually relate the text in this feature file to the tests themselves.

Every test scenario needs a `World` object. Often `World` holds state that is changing as Cucumber goes through each step in a scenario. The basic requirement for a `World` object is a `new()` function.

To enable testing of our Animal feature, add this code to `example.rs`:

```rust
use std::convert::Infallible;

use async_trait::async_trait;
use cucumber_rust::{given, World, WorldInit};

// These `Cat` definitions would normally be inside your project's code, but we 
// create them here to contain the test to just `cucumber.rs`.
#[derive(Debug)]
struct Cat {
    pub hungry: bool,
}

impl Cat {
    fn feed(&mut self) {
        self.hungry = false;
    }
}

// A World is your shared, likely mutable state.
#[derive(Debug, WorldInit)]
pub struct AnimalWorld {
    cat: Cat,
}

// `World` needs to be implemented, so Cucumber knows how to construct it on
// each `Scenario`.
#[async_trait(?Send)]
impl World for AnimalWorld {
    // We require some error type
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            cat: Cat { hungry: false },
        })
    }
}

// Steps are defined with `given`, `when` and `then` macros.
#[given("A hungry cat")]
fn hungry_cat(world: &mut AnimalWorld) {
    world.cat.hungry = true;
}

// This runs before everything else, so you can setup things here.
fn main() {
    // You may choose any executor you like (Tokio, async-std, etc)
    // You may even have an async main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(AnimalWorld::run("./tests/features/book"));
}
```

If you run this, you should see an output like:

[![Cucumber run with just the Given step](https://asciinema.org/a/DRnrcYrvPjRslamD1O1TxYxAc.svg)](https://asciinema.org/a/DRnrcYrvPjRslamD1O1TxYxAc)

You will see a checkmark next to "Given A hungry cat", which means that test step has been matched and executed.

But then for the next step: "I feed the cat", there is a "? ... (skipped)". This is because we have nothing in our steps that matches this sentence. The remaining steps in the scenario, since they depend on this skipped one, are not looked at all.

There are 3 types of steps:

- `given`, which is for defining the starting conditions and often initializing the data in the `World`
- `when`, for events or actions that are may trigger certain changes in the `World`
- `then`, to validate that the `World` has changed the way we would expect

These various `Step` functions are executed to transform the world. As such, mutable reference to the world must always be passed in. The step itself is also made available.

The steps functions take a string, which is the name of the given `Step` (i.e., the literal string, such as "A hungry cat"), and then a function closure that takes a `World` and then the `Step` itself. 

It can also take regex like that:

```rust,ignore
#[given(regex = r"^A hungry (\S+)$")]
fn hungry_someone(world: &mut AnimalWorld, who: String) {
    assert_eq!(who, "cat");
    world.cat.hungry = true;
}
```

We can add a `when` step after our `given` step:

```rust,ignore
// Don't forget to additionally `use cucumber_rust::when`.

#[when("I feed the cat")]
fn feed_cat(world: &mut AnimalWorld) {
    world.cat.feed();
}
```

If you run the tests again, you'll see that two lines are green now and the next one is marked as not yet implemented:

[![Cucumber run with a Given and When step](https://asciinema.org/a/M8QntIucnWUTyMydBmL1t8Os3.svg)](https://asciinema.org/a/M8QntIucnWUTyMydBmL1t8Os3)

Finally, how do we validate our result? We expect that this will cause some change in the cat and that the cat will no longer be hungry since it has been fed. The `then()` step follows to assert this, as our feature says:

```rust,ignore
// Don't forget to additionally `use cucumber_rust::then`.

#[then("The cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(!world.cat.hungry);
}
```

If you run the test now, you'll see that all steps are accounted for and the test succeeds:

[![Full Cucumber run](https://asciinema.org/a/MFWAj6dwMUL6JTP1Iji68qKHW.svg)](https://asciinema.org/a/MFWAj6dwMUL6JTP1Iji68qKHW)

If you want to be assured that your validation is indeed happening, you can change the assert for the cat being hungry from `true` to `false` temporarily:

```rust,ignore
#[then("The cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(world.cat.hungry);
}
```

And you should see the test fail:

[![Failing step](https://asciinema.org/a/4ZYqPERxMizgbc4Ztp6Khmjag.svg)](https://asciinema.org/a/4ZYqPERxMizgbc4Ztp6Khmjag)

What if we also wanted to validate that even if the cat was never hungry to begin with, it wouldn't end up hungry after it was fed? We can add another scenario that looks quite similar:

```gherkin
Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given A hungry cat
    When I feed the cat
    Then The cat is not hungry

  Scenario: If we feed a satiated cat it will not become hungry
    Given A satiated cat
    When I feed the cat
    Then The cat is not hungry

```

The only thing that is different is the Given. But we don't have to write a new function! We can leverage regex support:

```rust, ignore
#[given(regex = r"^A (hungry|satiated) cat$")]
fn hungry_cat(world: &mut AnimalWorld, state: String) {
    match state.as_str() {
        "hungry" =>  world.cat.hungry = true, 
        "satiated" =>  world.cat.hungry = false,
        _ => unreachable!(),
    }
}
```

We surround regex with `^..$` to unsure __exact__ match. This is much more useful as you add more and more steps, so they wouldn't interfere with each other.

Cucumber reuses the steps:

[![Steps reused between two scenarious](https://asciinema.org/a/UA6OiZWHW9RfXZ2wFSGXbdrqe.svg)](https://asciinema.org/a/UA6OiZWHW9RfXZ2wFSGXbdrqe)

A contrived example, but this demonstrates that steps can be reused as long as they are sufficiently precise in both their description and implementation. If, for example, the wording for our "Then" step was "The cat is no longer hungry", it'd imply something about the expected initial state, when that is not the purpose of a "Then" step, but rather of the "Given" step.

Full example so far:

```rust
use std::convert::Infallible;

use async_trait::async_trait;
use cucumber_rust::{given, then, when, World, WorldInit};

#[derive(Debug)]
struct Cat {
    pub hungry: bool,
}

impl Cat {
    fn feed(&mut self) {
        self.hungry = false;
    }
}

#[derive(Debug, WorldInit)]
pub struct AnimalWorld {
    cat: Cat,
}

#[async_trait(?Send)]
impl World for AnimalWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            cat: Cat { hungry: false },
        })
    }
}

#[given(regex = r"^A (hungry|satiated) cat$")]
fn hungry_cat(world: &mut AnimalWorld, state: String) {
    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when("I feed the cat")]
fn feed_cat(world: &mut AnimalWorld) {
    world.cat.feed();
}

#[then("The cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(!world.cat.hungry);
}

fn main() {
    futures::executor::block_on(AnimalWorld::run("./tests/features/book"));
}
```

## Asyncness

Let's play with `async` support a bit!

For that switch `futures` for `tokio` in dependencies:

```toml
[dev-dependencies]
async-trait = "0.1" # This is currently required to properly initialize the world in cucumber-rust
cucumber_rust = "0.10"
tokio = { version = "1.10", features = ["macros", "rt-multi-thread", "time"] }

[[test]]
name = "cucumber" # This should be the same as the filename of your test target
harness = false # Allows Cucumber to print output instead of libtest
```

And simply `sleep` on each step to test `async` support. In the real world you of course will switch it up to web/database requests, etc.

```rust
use std::{convert::Infallible, time::Duration};

use async_trait::async_trait;
use cucumber_rust::{given, then, when, World, WorldInit};
use tokio::time::sleep;

#[derive(Debug)]
struct Cat {
    pub hungry: bool,
}

impl Cat {
    fn feed(&mut self) {
        self.hungry = false;
    }
}

#[derive(Debug, WorldInit)]
pub struct AnimalWorld {
    cat: Cat,
}

#[async_trait(?Send)]
impl World for AnimalWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            cat: Cat { hungry: false },
        })
    }
}

#[given(regex = r"^A (hungry|satiated) cat$")]
async fn hungry_cat(world: &mut AnimalWorld, state: String) {
    sleep(Duration::from_secs(2)).await;

    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when("I feed the cat")]
async fn feed_cat(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    world.cat.feed();
}

#[then("The cat is not hungry")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}

#[tokio::main]
async fn main() {
    AnimalWorld::run("./tests/features/book/").await;
}
```

[![Async Cucumber](https://asciinema.org/a/GJjWi2Tn10jqNjWwnfOK3quad.svg)](https://asciinema.org/a/GJjWi2Tn10jqNjWwnfOK3quad)

Hm, it looks like executor waited only for the first `Feature` 🤔, what's going on?

By default `Cucumber` executes `Scenarios` [concurrently](https://en.wikipedia.org/wiki/Concurrent_computing)! That means executor actually did wait for all steps, but overlapped! This allows you to execute tests much faster!

If for some reason you don't want to run your `Scenarios` concurrently, use `@serial` tag on them:

```gherkin
Feature: Animal feature

  @serial
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given A hungry cat
    When I feed the cat
    Then The cat is not hungry

  @serial
  Scenario: If we feed a satiated cat it will not become hungry
    Given A satiated cat
    When I feed the cat
    Then The cat is not hungry
```

[![Async Cucumber with @serial tests](https://asciinema.org/a/xnt5WngXpwQBVBnxPjYD9NGS8.svg)](https://asciinema.org/a/xnt5WngXpwQBVBnxPjYD9NGS8)
