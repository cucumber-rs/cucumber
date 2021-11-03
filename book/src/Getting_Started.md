Getting Started
===============

Adding [Cucumber] to your project requires some groundwork. [Cucumber] tests are run along with other tests via `cargo test`, but rely on `.feature` files corresponding to the given test, as well as a set of step matchers described in code corresponding to the steps in those `.feature` files.

To start, create a directory called `tests/` in the root of your project and add a file to represent your test target (in this walkthrough we use `example.rs`).

Add this to your `Cargo.toml`:
```toml
[dev-dependencies]
async-trait = "0.1"
cucumber = "0.10"
futures = "0.3"

[[test]]
name = "example" # this should be the same as the filename of your test target
harness = false  # allows Cucumber to print output instead of libtest
```

At this point, while it won't do anything, you should be able to successfully run `cargo test --test example` without errors, as long as your `example.rs` has at least a `main()` function defined.

Create a directory to store `.feature` files somewhere in your project (in this walkthrough we use `tests/features/book/` directory), and put a `.feature` file there (such as `animal.feature`). This should contain the [Gherkin] spec for a scenario that you want to test. Here's a very simple example:
```gherkin
Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry
```

Here is how we actually relate the text in this `.feature` file to the tests themselves: every test scenario needs a `World` object. Often `World` holds a state that is changing as [Cucumber] goes through each step in a scenario. The basic requirement for a `World` object is a `new()` function.

To enable testing of our `animal.feature`, add this code to `example.rs`:
```rust
use std::convert::Infallible;

use async_trait::async_trait;
use cucumber::{given, World, WorldInit};

// These `Cat` definitions would normally be inside your project's code, 
// not test code, but we create them here for the show case.
#[derive(Debug)]
struct Cat {
    pub hungry: bool,
}

impl Cat {
    fn feed(&mut self) {
        self.hungry = false;
    }
}

// `World` is your shared, likely mutable state.
#[derive(Debug, WorldInit)]
pub struct AnimalWorld {
    cat: Cat,
}

// `World` needs to be implemented, so Cucumber knows how to construct it on
// each `Scenario`.
#[async_trait(?Send)]
impl World for AnimalWorld {
    // We require some error type.
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            cat: Cat { hungry: false },
        })
    }
}

// Steps are defined with `given`, `when` and `then` macros.
#[given("a hungry cat")]
fn hungry_cat(world: &mut AnimalWorld) {
    world.cat.hungry = true;
}

// This runs before everything else, so you can setup things here.
fn main() {
    // You may choose any executor you like (`tokio`, `async-std`, etc.).
    // You may even have an `async` main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(AnimalWorld::run("/tests/features/book"));
}
```

If you run this, you should see an output like:

<script id="asciicast-loqmDmLvKdp4CG7URpVsLJgkB" src="https://asciinema.org/a/loqmDmLvKdp4CG7URpVsLJgkB.js" async data-autoplay="true" data-rows="23"></script>

You will see a checkmark next to `Given A hungry cat`, which means that test step has been matched and executed.

But then for the next step `I feed the cat` there is a `? ... (skipped)`. This is because we have nothing in our steps that matches this sentence. The remaining steps in the scenario, since they depend on this skipped one, are not looked and run at all.

There are 3 types of steps:
- `given`: for defining the starting conditions and often initializing the data in the `World`;
- `when`: for events or actions that are may trigger certain changes in the `World`;
- `then`: to validate that the `World` has changed the way we would expect.

These various `Step` functions are executed to transform the `World`. As such, mutable reference to the world must always be passed in. The `Step` itself is also made available.

The steps matchers take a string, which is the name of the given `Step` (i.e., the literal string, such as `A hungry cat`), and then a function closure that takes a `World` and then the `Step` itself.

We can add a `when` step after our `given` step:
```rust
# use std::convert::Infallible;
# 
# use async_trait::async_trait;
# use cucumber::{given, when, World, WorldInit};
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
# #[given("a hungry cat")]
# fn hungry_cat(world: &mut AnimalWorld) {
#     world.cat.hungry = true;
# }
# 
// Don't forget to additionally `use cucumber::when;`.

#[when("I feed the cat")]
fn feed_cat(world: &mut AnimalWorld) {
    world.cat.feed();
}
#
# fn main() {
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book"));
# }
```

If you run the tests again, you'll see that two lines are green now and the next one is marked as not yet implemented:

<script id="asciicast-iyhXabbOv7jdKvbcsyhzqPMfo" src="https://asciinema.org/a/iyhXabbOv7jdKvbcsyhzqPMfo.js" async data-autoplay="true" data-rows="15"></script>

Finally: how do we validate our result? We expect that this will cause some change in the cat and that the cat will no longer be hungry since it has been fed. The `then()` step follows to assert this, as our feature says:
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
# #[given("a hungry cat")]
# fn hungry_cat(world: &mut AnimalWorld) {
#     world.cat.hungry = true;
# }
#
# #[when("I feed the cat")]
# fn feed_cat(world: &mut AnimalWorld) {
#     world.cat.feed();
# }
#
// Don't forget to additionally `use cucumber::then;`.

#[then("the cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(!world.cat.hungry);
}
#
# fn main() {
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book"));
# }
```

If you run the test now, you'll see that all steps are accounted for and the test succeeds:

<script id="asciicast-fHuIXkWrIk1AOFFqF0MYmY0m0" src="https://asciinema.org/a/fHuIXkWrIk1AOFFqF0MYmY0m0.js" async data-autoplay="true" data-rows="16"></script>

In addition to assertions, you can also return a `Result<()>` from your step function. Returning `Err` will cause the step to fail. This lets you use the `?` operator for more concise step implementations just like in [unit tests](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#tests-and-).

If you want to be assured that your validation is indeed happening, you can change the assertion for the cat being hungry from `true` to `false` temporarily:
```rust,should_panic
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
#     // We require some error type
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Cat { hungry: false },
#         })
#     }
# }
#
# #[given("a hungry cat")]
# fn hungry_cat(world: &mut AnimalWorld) {
#     world.cat.hungry = true;
# }
#
# #[when("I feed the cat")]
# fn feed_cat(world: &mut AnimalWorld) {
#     world.cat.feed();
# }
#
#[then("the cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(world.cat.hungry);
}
# fn main() {
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book"));
# }
```

And you should see the test failing:

<script id="asciicast-XTikmqirO7mAFZ97MNfKvnD5p" src="https://asciinema.org/a/XTikmqirO7mAFZ97MNfKvnD5p.js" async data-autoplay="true" data-rows="24"></script>

What if we also wanted to validate that even if the cat was never hungry to begin with, it wouldn't end up hungry after it was fed? We can add another scenario that looks quite similar:
```gherkin
Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry

  Scenario: If we feed a satiated cat it will not become hungry
    Given a satiated cat
    When I feed the cat
    Then the cat is not hungry

```

The only thing that is different is the `Given` step. But we don't have to write a new matcher! We can leverage regex support:
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
#[given(regex = r"^a (hungry|satiated) cat$")]
fn hungry_cat(world: &mut AnimalWorld, state: String) {
    match state.as_str() {
        "hungry" =>  world.cat.hungry = true,
        "satiated" =>  world.cat.hungry = false,
        _ => unreachable!(),
    }
}
#
# #[when("I feed the cat")]
# fn feed_cat(world: &mut AnimalWorld) {
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# fn cat_is_fed(world: &mut AnimalWorld) {
#     assert!(!world.cat.hungry);
# }
#
# fn main() {
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book"));
# }
```

We surround regex with `^..$` to ensure the __exact__ match. This is much more useful as you add more and more steps, so they wouldn't interfere with each other.

[Cucumber] will reuse these steps:

<script id="asciicast-ao6LdWsrtdsgg8tOi9cQgPfyz" src="https://asciinema.org/a/ao6LdWsrtdsgg8tOi9cQgPfyz.js" async data-autoplay="true" data-rows="18"></script>

Captured groups are __bold__ to indicate which part of step could be dynamically changed.  

A contrived example, but this demonstrates that steps can be reused as long as they are sufficiently precise in both their description and implementation. If, for example, the wording for our `Then` step was `The cat is no longer hungry`, it'd imply something about the expected initial state, when that is not the purpose of a `Then` step, but rather of the `Given` step.

<details>
<summary>Full example so far:</summary>
<br>

```rust
use std::convert::Infallible;

use async_trait::async_trait;
use cucumber::{given, then, when, World, WorldInit};

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

#[given(regex = r"^a (hungry|satiated) cat$")]
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

#[then("the cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(!world.cat.hungry);
}

fn main() {
    futures::executor::block_on(AnimalWorld::run("/tests/features/book"));
}
```
</details>




## Asyncness

Let's play with `async` support a bit!

For that switch `futures` for `tokio` in dependencies:

```toml
[dev-dependencies]
async-trait = "0.1"
cucumber = "0.10"
tokio = { version = "1.10", features = ["macros", "rt-multi-thread", "time"] }

[[test]]
name = "cucumber" # this should be the same as the filename of your test target
harness = false   # allows Cucumber to print output instead of libtest
```

And simply `sleep` on each step to test the `async` support. In the real world you of course will switch it up to web/database requests, etc.
```rust
# use std::{convert::Infallible, time::Duration};
# 
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
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
#[given(regex = r"^a (hungry|satiated) cat$")]
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

#[then("the cat is not hungry")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}

#[tokio::main]
async fn main() {
    AnimalWorld::run("/tests/features/book").await;
}
```

<script id="asciicast-tz9ApYZgsET9k8jjIa9HpnQ8p" src="https://asciinema.org/a/tz9ApYZgsET9k8jjIa9HpnQ8p.js" async data-autoplay="true" data-rows="18"></script>

Hm, it looks like the executor waited only for the first `Feature` 🤔, what's going on?

By default `Cucumber` executes `Scenarios` [concurrently](https://en.wikipedia.org/wiki/Concurrent_computing)! That means executor actually did wait for all the steps, but overlapped! This allows you to execute tests much faster!

If for some reason you don't want to run your `Scenarios` concurrently, use `@serial` tag on them:

```gherkin
Feature: Animal feature

  @serial
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry

  @serial
  Scenario: If we feed a satiated cat it will not become hungry
    Given a satiated cat
    When I feed the cat
    Then the cat is not hungry
```

<script id="asciicast-MDXpZf8vcOTssmlU4rgSSfR0E" src="https://asciinema.org/a/MDXpZf8vcOTssmlU4rgSSfR0E.js" async data-autoplay="true" data-rows="18"></script>




[Cucumber]: https://cucumber.io
[Gherkin]: https://cucumber.io/docs/gherkin/reference
