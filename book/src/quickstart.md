Quickstart
==========

Adding [Cucumber] to a project requires some groundwork. [Cucumber] tests are run along with other tests via `cargo test`, but rely on `.feature` files corresponding to the given test, as well as a set of [step] matchers (described in code) corresponding to the [step]s in those `.feature` files.

To start, let's create a directory called `tests/` in the root of the project and add a file to represent the test target (in this walkthrough it's `example.rs`).

Add this to `Cargo.toml`:
```toml
[dev-dependencies]
async-trait = "0.1"
cucumber = "0.10"
futures = "0.3"

[[test]]
name = "example" # this should be the same as the filename of your test target
harness = false  # allows Cucumber to print output instead of libtest
```

At this point, while it won't do anything, it should successfully run `cargo test --test example` without errors, as long as the `example.rs` file has at least a `main()` function defined.

Now, let's create a directory to store `.feature` files somewhere in the project (in this walkthrough it's `tests/features/book/` directory), and put a `.feature` file there (such as `animal.feature`). It should contain a [Gherkin] spec for the [scenario] we want to test. Here's a very simple example:
```gherkin
Feature: Animal feature

  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry
```

To relate the text of the `.feature` file with the actual tests we would need a `World` object, holding a state that is newly created for each [scenario] and is changing as [Cucumber] goes through each [step] of that [scenario]. The basic requirement for a `World` object is to provide a `new()` function.

To enable testing of our `animal.feature`, let's add this code to `example.rs`:
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

// `World` needs to be implemented, so Cucumber knows how to construct it
// for each scenario.
#[async_trait(?Send)]
impl World for AnimalWorld {
    // We do require some error type.
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            cat: Cat { hungry: false },
        })
    }
}

// Steps are defined with `given`, `when` and `then` attributes.
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

If we run this, we should see an output like this:  
![record](rec/quickstart_simple_1.gif)

A checkmark `✔` next to the `Given a hungry cat` [step] means that it has been matched, executed and passed.

But then, for the next `When I feed the cat` [step] there is a question mark `?`, meaning that we have nothing in our tests matching this sentence. The remaining [step]s in the [scenario] are not looked and run at all, since they depend on the skipped one.

There are 3 types of [step]s:
- `given`: for defining [scenario] starting conditions and often initializing the data in the `World`;
- `when`: for events or actions triggering the tested changes in the `World` representing the [scenario];
- `then`: to validate that the `World` has changed in the way expected by the [scenario].

These various [step] matching functions are executed to transform the `World`. As such, mutable reference to the world must always be passed in. The `Step` itself is also made available.

We can add a `when` [step] matcher:
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
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book/quickstart/simple.feature"));
# }
```

Once we run the tests again, we see that two lines are green now and the next one is marked as not yet implemented:  
![record](rec/quickstart_simple_2.gif)

Finally, how do we check our result? We expect that this will cause some change in the cat and that the cat will no longer be hungry since it has been fed. The `then` [step] matcher follows to assert this, as our [feature] says:
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
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book/quickstart/simple.feature"));
# }
```

Once we run the tests, now we see all steps being accounted for and the whole [scenario] passing:  
![record](rec/quickstart_simple_3.gif)

> __TIP__: In addition to assertions, we may also return a `Result<()>` from a [step] matching function. Returning `Err` will cause the [step] to fail. This lets using the `?` operator for more concise step implementations just like in [unit tests](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#tests-and-).

To assure that assertion is indeed happening, let's reverse it temporarily:
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
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book/quickstart/simple.feature"));
# }
```

And see the test failing:  
![record](rec/quickstart_simple_fail.gif)

What if we also want to validate that even if the cat was never hungry to begin with, it won't end up hungry after it was fed? So, we may add an another [scenario] that looks quite similar:
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

The only thing that is different is the `Given` [step]. But we don't have to write a new matcher here! We can leverage [`regex`] support:
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
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book/quickstart/concurrent.feature"));
# }
```

> __NOTE__: We surround the regex with `^..$` to ensure an __exact__ match. This is much more useful when adding more and more [step]s, so they won't accidentally interfere with each other.

[Cucumber] will reuse these [step] matchers:  
![record](rec/quickstart_concurrent_sync.gif)

> __NOTE__: Captured groups are __bold__ to indicate which part of a [step] could be dynamically changed.

Alternatively, we also may use [Cucumber Expressions] for the same purpose (less powerful, but much more readable):
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
#[given(expr = "a {word} cat")]
fn hungry_cat(world: &mut AnimalWorld, state: String) {
    match state.as_str() {
        "hungry" =>  world.cat.hungry = true,
        "satiated" =>  world.cat.hungry = false,
        s => panic!("expected 'hungry' or 'satiated', found: {}", s),
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
#     futures::executor::block_on(AnimalWorld::run("/tests/features/book/quickstart/simple.feature"));
# }
```

A contrived example, but it demonstrates that [step]s can be reused as long as they are sufficiently precise in both their description and implementation. If, for example, the wording for our `Then` [step] was `The cat is no longer hungry`, it would imply something about the expected initial state, when that is not the purpose of a `Then` [step], but rather of the `Given` [step].




## Asyncness

`async` execution is supported naturally.

Let's switch our runtime to `tokio`:
```toml
[dev-dependencies]
async-trait = "0.1"
cucumber = "0.10"
tokio = { version = "1.10", features = ["macros", "rt-multi-thread", "time"] }

[[test]]
name = "example" # this should be the same as the filename of your test target
harness = false  # allows Cucumber to print output instead of libtest
```

And, simply `sleep` on each [step] to test the `async` support (in the real world, of course, there will be web/database requests, etc.):
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
![record](rec/quickstart_concurrent_async.gif)

Hm, it looks like the runtime waited only for the first `Feature`, while the second was printed instantly. What's going on? 🤔

By default, [Cucumber] executes [scenario]s [concurrently](https://en.wikipedia.org/wiki/Concurrent_computing)! That means that runtime actually did wait for all the [step]s, but overlapped! This allows us to execute tests much faster!

If for some reason we don't want to run [scenario]s concurrently, we may use `@serial` [tag] on them:
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
![record](rec/quickstart_serial.gif)

> __NOTE__: Any [scenario] marked with `@serial` will be executed in isolation, ensuring that there are no other [scenario]s running concurrently.

> __TIP__: To run the whole test suite serially, consider using `--concurrency=1` [CLI] option, rather than marking evey single [feature] with a `@serial` tag.




[`regex`]: https://docs.rs/regex

[CLI]: https://en.wikipedia.org/wiki/Command-line_interface
[Cucumber]: https://cucumber.io
[Cucumber Expressions]: https://cucumber.github.io/cucumber-expressions
[feature]: https://cucumber.io/docs/gherkin/reference#feature
[Gherkin]: https://cucumber.io/docs/gherkin/reference
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[step]: https://cucumber.io/docs/gherkin/reference#steps
[tag]: https://cucumber.io/docs/cucumber/api#tags
