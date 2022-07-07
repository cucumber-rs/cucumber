Asserting
=========

There are two ways of doing [assertion]s in a [step] matching function: 
- [throwing a panic](#panic);
- [returning an error](#result-and-).




## Panic

Throwing a panic in a [step] matching function makes the appropriate [step] failed:
```rust,should_panic
# use cucumber::{given, then, when, World};
#
# #[derive(Debug, Default)]
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
# #[derive(Debug, Default, World)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     match state.as_str() {
#         "hungry" =>  world.cat.hungry = true,
#         "satiated" =>  world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# fn feed_cat(world: &mut AnimalWorld) {
#     world.cat.feed();
# }
#
#[then("the cat is not hungry")]
fn cat_is_fed(_: &mut AnimalWorld) {
    panic!("Cats are always hungry!")
}
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::cucumber()
#         .run_and_exit("tests/features/book/writing/asserting.feature")
#         .await;
# }
```
![record](../rec/writing_asserting_panic.gif)

> __NOTE__: Failed [step] prints its location in a `.feature` file and the captured [assertion] message.

> __TIP__: To additionally print the state of the `World` at the moment of failure, increase output verbosity via `-vv` [CLI] option.

> __TIP__: By default, unlike [unit tests](https://doc.rust-lang.org/cargo/commands/cargo-test.html#test-options), failed [step]s don't terminate the execution instantly, and the whole test suite is executed regardless of them. Use `--fail-fast` [CLI] option to stop execution on first failure.




## `Result` and `?`

Similarly to [using the `?` operator in Rust tests][1], we may also return a `Result<()>` from a [step] matching function, so returning an `Err` will cause the [step] to fail (anything implementing [`Display`] is sufficient).
```rust,should_panic
# use cucumber::{given, then, when, World};
#
# #[derive(Debug, Default)]
# struct Cat {
#     pub hungry: bool,
# }
#
# #[derive(Debug, Default, World)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     match state.as_str() {
#         "hungry" =>  world.cat.hungry = true,
#         "satiated" =>  world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
#[when("I feed the cat")]
fn feed_cat(_: &mut AnimalWorld) {}

#[then("the cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) -> Result<(), &'static str> {
    (!world.cat.hungry).then_some(()).ok_or("Cat is still hungry!")
}
#
# #[tokio::main]
# async fn main() {
#     AnimalWorld::cucumber()
#         .run_and_exit("tests/features/book/writing/asserting.feature")
#         .await;
# }
```
![record](../rec/writing_asserting_result.gif)




[`Display`]: https://doc.rust-lang.org/stable/std/fmt/trait.Display.html
[assertion]: https://en.wikipedia.org/wiki/Assertion_(software_development)
[CLI]: ../cli.md
[step]: https://cucumber.io/docs/gherkin/reference#steps
[1]: https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#tests-and-
