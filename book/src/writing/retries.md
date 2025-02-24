Retrying failed scenarios
=========================

Often, it's nearly impossible to create fully-deterministic test case, especially when you are relying on environments like external services, browsers, file system, networking etc. That's why there is an ability to retry failed [scenario]s. 

> __WARNING__: Although this feature is supported, we highly recommend to use it as the _last resort only_. First, consider implementing in-[step] retries with your own needs (like [exponential backoff]). Other ways of dealing with flaky tests include, but not limited to: reducing number of concurrently executed scenarios (maybe even using `@serial` [tag]), mocking external environment, [controlling time in tests] or even [simulation testing]. It's always better to move towards tests determinism, rather than trying to tame their flakiness.




## Tags

Recommended way to specify retried [scenario]s is using [tags][tag] ([inheritance] is supported too):
```gherkin
Feature: Heads and tails

  # Attempts a single retry immediately.
  @retry
  Scenario: Tails
    Given a coin
    When I flip the coin
    Then I see tails
      
  # Attempts a single retry in 1 second.
  @retry.after(1s)
  Scenario: Heads
    Given a coin
    When I flip the coin
    Then I see heads

  # Attempts to retry 5 times with no delay between them.
  @retry(5)
  Scenario: Edge
    Given a coin
    When I flip the coin
    Then I see edge

  # Attempts to retry 10 times with 100 milliseconds delay between them.
  @retry(10).after(100ms)
  Scenario: Levitating
    Given a coin
    When I flip the coin
    Then the coin never lands
```
```rust,should_panic
# extern crate cucumber;
# extern crate rand;
# extern crate tokio;
#
# use std::time::Duration;
#
# use cucumber::{World, given, then, when};
# use rand::Rng as _;
# use tokio::time::sleep;
#
# #[derive(Debug, Default, World)]
# pub struct FlipWorld {
#     flipped: &'static str,
# }
#
#[given("a coin")]
async fn coin(_: &mut FlipWorld) {
    sleep(Duration::from_secs(2)).await;
}

#[when("I flip the coin")]
async fn flip(world: &mut FlipWorld) {
    sleep(Duration::from_secs(2)).await;

    world.flipped = match rand::thread_rng().gen_range(0.0..1.0) {
        p if p < 0.2 => "edge",
        p if p < 0.5 => "heads",
        _ => "tails",
    }
}

#[then(regex = r#"^I see (heads|tails|edge)$"#)]
async fn see(world: &mut FlipWorld, what: String) {
    sleep(Duration::from_secs(2)).await;

    assert_eq!(what, world.flipped);
}

#[then("the coin never lands")]
async fn never_lands(_: &mut FlipWorld) {
    sleep(Duration::from_secs(2)).await;

    unreachable!("coin always lands")
}
#
# #[tokio::main]
# async fn main() {
#     FlipWorld::cucumber()
#         .fail_on_skipped()
#         .run_and_exit("tests/features/book/writing/retries.feature")
#         .await;
# }
```
![record](../rec/writing_retries.gif)

> __NOTE__: On failure, the whole [scenario] is re-executed with a new fresh [`World`] instance. 




## CLI

The following [CLI option]s are related to the [scenario] retries:
```text
--retry <int>
    Number of times a scenario will be retried in case of a failure

--retry-after <duration>
    Delay between each scenario retry attempt.
    
    Duration is represented in a human-readable format like `12min5s`.
    Supported suffixes:
    - `nsec`, `ns` — nanoseconds.
    - `usec`, `us` — microseconds.
    - `msec`, `ms` — milliseconds.
    - `seconds`, `second`, `sec`, `s` - seconds.
    - `minutes`, `minute`, `min`, `m` - minutes.

--retry-tag-filter <tagexpr>
    Tag expression to filter retried scenarios
```

- `--retry` [CLI option] is similar to `@retry(<number-of-retries>)` [tag], but is applied to all [scenario]s matching the `--retry-tag-filter` (if not provided, all possible [scenario]s are matched).
- `--retry-after` [CLI option] is similar to `@retry.after(<delay-after-each-retry>)` [tag] in the same manner.


### Precedence of tags and CLI options

- Just `@retry` [tag] takes the number of retries and the delay from `--retry` and `--retry-after` [CLI option]s respectively, if they're specified, otherwise defaults to a single retry attempt with no delay.
- `@retry(3)` [tag] always retries failed [scenario]s at most 3 times, even if `--retry` [CLI option] provides a greater value. Delay is taken from `--retry-after` [CLI option], if it's specified, otherwise defaults to no delay.
- `@retry.after(1s)` [tag] always delays 1 second before next retry attempt, even if `--retry-after` [CLI option] provides another value. Number of retries is taken from `--retry-after` [CLI option], if it's specified, otherwise defaults a single retry attempt.
- `@retry(3).after(1s)` always retries failed scenarios at most 3 times with 1 second delay before each attempt, ignoring `--retry` and `--retry-after` [CLI option]s.

> __NOTE__: When using with `--fail-fast` [CLI option] (or [`.fail_fast()` builder config][1]), [scenario]s are considered as failed only in case they exhaust all retry attempts and then still do fail.

> __TIP__: It could be handy to specify `@retry` [tags][tag] only, without any explicit values, and use `--retry=n --retry-after=d --retry-tag-filter=@retry` [CLI option]s to overwrite retrying parameters without affecting any other [scenario]s.




[`World`]: https://docs.rs/cucumber/latest/cucumber/trait.World.html
[CLI option]: ../cli.md
[controlling time in tests]: https://docs.rs/tokio/1.0/tokio/time/fn.pause.html
[exponential backoff]: https://en.wikipedia.org/wiki/Exponential_backoff
[inheritance]: tags.md#inheritance
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[simulation testing]: https://github.com/madsys-dev/madsim
[step]: https://cucumber.io/docs/gherkin/reference#steps
[tag]: https://cucumber.io/docs/cucumber/api#tags

[1]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.fail_fast
