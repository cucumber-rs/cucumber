Terminal output
===============

By [default][1], [`cucumber`] crate outputs tests result to [STDOUT]. It provides some [CLI options][2] for configuring the output.




## Verbosity

By [default][1], [`cucumber`] crate omits outputting [doc strings][doc] of [step]s. To include them into the output use `--verbose` CLI option:
```gherkin
Feature: Animal feature
    
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
      """
      A hungry cat called Felix is rescued from a Whiskas tin in a calamitous 
      mash-up of cat food brands.
      """
    When I feed the cat
    Then the cat is not hungry
```
```rust
# use std::{convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
#[tokio::main]
async fn main() {
    AnimalWorld::cucumber()
        .run_and_exit("/tests/features/book/output/terminal_verbose.feature")
        .await;
}
```
![record](../rec/output_terminal_verbose.gif)




## Coloring

Coloring may be disabled by specifying `--color` CLI option:
```rust
# use std::{convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
#[tokio::main]
async fn main() {
    AnimalWorld::cucumber()
        .run_and_exit("/tests/features/book/output/terminal.feature")
        .await;
}
```
![record](../rec/output_terminal_color.gif)

> __NOTE__: By [default][1], [`cucumber`] crate automatically disables coloring for non-interactive terminals, so there is no need to specify `--color` CLI option explicitly on [CI].




## Manual printing

Though [`cucumber`] crate doesn't capture any manual printing produced in a [step] matching function (such as [`dbg!`] or [`println!`] macros), it may be [quite misleading][#177] to produce and use it for debugging purposes. The reason is simply because [`cucumber`] crate executes [scenario]s concurrently and [normalizes][3] their results before outputting, while any manual print is produced instantly at the moment of its [step] execution.

> __WARNING:__ Moreover, manual printing will very likely interfere with [default][1] interactive pretty-printing.

```rust
# use std::{convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
#[when("I feed the cat")]
async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
    dbg!("here!");
    world.cat.feed();
}
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::cucumber()
#         .run_and_exit("/tests/features/book/output/terminal.feature")
#         .await;
# }
```
![record](../rec/output_terminal_custom_bad.gif)

To achieve natural output for debugging, the following preparations are required:
1. Setting [`.max_concurrent_scenarios()`] to `1` for executing all the [scenario]s sequentially.
2. Creating [`writer::Basic::raw`] with [`Coloring::Never`] to avoid interactive pretty-printed output.
3. Wrapping it into [`writer::AssertNormalized`] to assure [`cucumber`] about the output being [normalized][4] already (due to sequential execution).

```rust
# use std::{convert::Infallible, io, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, writer, World, WorldInit, WriterExt as _};
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#     dbg!("here!");    
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
#[tokio::main]
async fn main() {
    AnimalWorld::cucumber()
        .max_concurrent_scenarios(1)
        .with_writer(
            writer::Basic::raw(io::stdout(), writer::Coloring::Never, false)
                .summarized()
                .assert_normalized(),
        )
        .run_and_exit("/tests/features/book/output/terminal.feature")
        .await;
}
```
![record](../rec/output_terminal_custom.gif)

> __NOTE__: The custom print is still output before its [step], because is printed during the [step] execution. 




## Repeating failed and/or skipped [step]s

As a number of [scenario]s grows, it may become quite difficult to find failed/skipped ones in a large output. This issue may be mitigated by duplicating failed and/or skipped [step]s at the and of output via [`Cucumber::repeat_failed()`] and [`Cucumber::repeat_skipped()`] methods respectively.

```rust,should_panic
# use std::{convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
#[tokio::main]
async fn main() {
    AnimalWorld::cucumber()
        .repeat_failed()
        .run_and_exit("/tests/features/book/output/terminal_repeat_failed.feature")
        .await;
}
```
![record](../rec/output_terminal_repeat_failed.gif)

```rust
# use std::{convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
#[tokio::main]
async fn main() {
    AnimalWorld::cucumber()
        .repeat_skipped()
        .run_and_exit("/tests/features/book/output/terminal_repeat_skipped.feature")
        .await;
}
```
![record](../rec/output_terminal_repeat_skipped.gif)




[#177]: https://github.com/cucumber-rs/cucumber/issues/177
[`.max_concurrent_scenarios()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.max_concurrent_scenarios 
[`Coloring::Never`]: https://docs.rs/cucumber/*/cucumber/writer/enum.Coloring.html#variant.Never
[`cucumber`]: https://docs.rs/cucumber
[`Cucumber::repeat_failed()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.repeat_failed
[`Cucumber::repeat_skipped()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.repeat_skipped
[`dbg!`]: https://doc.rust-lang.org/stable/std/macro.dbg.html 
[`println!`]: https://doc.rust-lang.org/stable/std/macro.println.html
[`writer::AssertNormalized`]: https://docs.rs/cucumber/*/cucumber/writer/struct.AssertNormalized.html
[`writer::Basic::raw`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Basic.html#method.raw
[CI]: https://en.wikipedia.org/wiki/Continuous_integration
[doc]: https://cucumber.io/docs/gherkin/reference#doc-strings
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[STDOUT]: https://en.wikipedia.org/wiki/Standard_streams#Standard_output_(stdout)
[step]: https://cucumber.io/docs/gherkin/reference#steps
[1]: https://docs.rs/cucumber/*/cucumber/writer/struct.Basic.html
[2]: https://docs.rs/cucumber/*/cucumber/writer/basic/struct.Cli.html
[3]: https://docs.rs/cucumber/*/cucumber/writer/struct.Normalize.html
[4]: https://docs.rs/cucumber/*/cucumber/writer/trait.Normalized.html
