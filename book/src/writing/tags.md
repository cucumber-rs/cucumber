Tags
====

[Tags][tag] represent meta information of [scenario]s and [feature]s.

They can be used for different purposes, but in the majority of cases it's just:
- either running a subset of [scenario]s filtering by [tag];
- or making [scenario] run in isolation via `@serial` [tag];
- or allowing [scenario]s to be skipped with `@allow.skipped` [tag].




## Filtering

A [scenario] may have as many [tag]s as it requires (they should be separated with spaces):
```gherkin
Feature: Animal feature

  @hungry
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry

  @satiated @second
  Scenario: If we feed a satiated cat it will not become hungry
    Given a satiated cat
    When I feed the cat
    Then the cat is not hungry
```

To filter out running [scenario]s we may use:
- either `--tags` [CLI] option providing [tag expressions] (also consider [escaping]);
- or [`filter_run()`]-like method.

![record](../rec/writing_tags_filtering.gif)




## Inheritance

[Tags][tag] may be placed above the following [Gherkin] elements:
- [`Feature`][feature]
- [`Rule`][rule]
- [`Scenario`][scenario]
- [`Scenario Outline`]
- [`Examples`]

It's _not_ possible to place [tag]s above [`Background`](background.md) or [step]s (`Given`, `When`, `Then`, `And` and `But`).

[Tags][tag] are inherited by child elements:
- [`Feature`][feature] and [`Rule`][rule] [tag]s will be inherited by [`Scenario`][scenario], [`Scenario Outline`], or [`Examples`].
- [`Scenario Outline`] [tag]s will be inherited by [`Examples`].

```gherkin
@feature
Feature: Animal feature

  @scenario
  Scenario Outline: If we feed a hungry animal it will no longer be hungry
    Given a hungry <animal>
    When I feed the <animal> <n> times
    Then the <animal> is not hungry

  @home
  Examples: 
    | animal | n |
    | cat    | 2 |
    | dog    | 3 |

  @dire
  Examples: 
    | animal | n |
    | lion   | 1 |
    | wolf   | 1 |
```

> __NOTE__: In [`Scenario Outline`] it's possible to use [tag]s on different [`Examples`].

![record](../rec/writing_tags_inheritance.gif)




## Isolated execution

[`cucumber`] crate provides out-of-the-box support for `@serial` [tag]. Any [scenario] marked with `@serial` [tag] will be executed in isolation, ensuring that there are no other [scenario]s running concurrently at the moment.

```gherkin
Feature: Animal feature
    
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

> __NOTE__: `@serial` [tag] may also be used for filtering as a regular one.

![record](../rec/writing_tags_serial.gif)

> __TIP__: To run the whole test suite serially, consider using `--concurrency=1` [CLI] option, rather than marking evey single [feature] with a `@serial` [tag].




## Failing on skipped [step]s

As a test suit grows, it may become harder to notice how minimal changes to [regular expressions](capturing.md) can lead to mismatched [step]s.

Using [`Cucumber::fail_on_skipped()`] method fails the whole test suite if some [step]s miss the implementation, so ensures that the whole test suite is covered.

```gherkin
Feature: Animal feature
    
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry

  Scenario: If we feed a satiated cat it will not become hungry
    Given a wild cat
    When I feed the cat
    Then the cat is not hungry
```
```rust,should_panic
# use std::time::Duration;
#
# use cucumber::{given, then, when, World};
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
# #[derive(Debug, Default, World)]
# pub struct AnimalWorld {
#     cat: Animal,
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
        .fail_on_skipped()
        .run_and_exit("tests/features/book/writing/tags_skip_failed.feature")
        .await;
}
```

> __TIP__: Using `@allow.skipped` [tag] allows [scenario]s being skipped even in [`Cucumber::fail_on_skipped()`] mode. Use the one to intentionally skip the implementation.

```gherkin
Feature: Animal feature
    
  Scenario: If we feed a hungry cat it will no longer be hungry
    Given a hungry cat
    When I feed the cat
    Then the cat is not hungry

  @allow.skipped
  Scenario: If we feed a satiated cat it will not become hungry
    Given a wild cat
    When I feed the cat
    Then the cat is not hungry
```

![record](../rec/writing_tags_skip.gif)

> __NOTE__: `@allow.skipped` [tag] may also be used for filtering as a regular one.

![record](../rec/writing_tags_skip_filter.gif)




[`cucumber`]: https://docs.rs/cucumber
[`Cucumber::fail_on_skipped()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.fail_on_skipped
[`Examples`]: https://cucumber.io/docs/gherkin/reference#examples
[`filter_run()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.filter_run
[`Scenario Outline`]: scenario_outline.md
[CLI]: ../cli.md
[escaping]: https://github.com/cucumber/tag-expressions/tree/6f444830b23bd8e0c5a2617cd51b91bc2e05adde#escaping
[feature]: https://cucumber.io/docs/gherkin/reference#feature
[Gherkin]: https://cucumber.io/docs/gherkin/reference
[rule]: https://cucumber.io/docs/gherkin/reference#rule
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[step]: https://cucumber.io/docs/gherkin/reference#steps
[tag]: https://cucumber.io/docs/cucumber/api#tags
[tag expressions]: https://cucumber.io/docs/cucumber/api#tag-expressions
