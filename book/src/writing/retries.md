Retrying failed scenarios
=========================

Often it's nearly impossible to create fully-deterministic test case, especially when you are relying on environment like external services, browsers, file system, networking etc. This is why this crate has an ability to retry failed scenarios. Although this feature is supported, it should be used as a last resort. Consider implementing in-step retries with your own needs, like exponential backoff, first. Other ways of dealing with flaky tests include, but not limited to: reducing number of concurrently executed scenarios (maybe even using `@serial` tag), mocking external environment, [controlling time in tests] or even [simulation testing].




## Tags

Recommended way to specify retried scenarios is using tags:

```gherkin
Feature: Heads and tails

  @retry
  Scenario: Tails
    Given a coin
    When I flip the coin
    Then I see tails

  @retry.after(1s)
  Scenario: Heads
      Given a coin
      When I flip the coin
      Then I see heads

  @retry(5)
  Scenario: Edge
      Given a coin
      When I flip the coin
      Then I see edge

  @retry(100).after(100ms)
  Scenario: Levitating
      Given a coin
      When I flip the coin
      Then the coin never lands
```

> __NOTE__: On failure, whole Scenario is re-executed with a new fresh [`World`] instance. 

Tag syntax is following: `@retry(<number-of-retries>).after(<delay-after-each-retry>)`. Number of retries and delay can be omitted, in that case they will default to `1` and `0s` (or values defined in the CLI)




## CLI

The following CLI options are related to the retries

```
--retry <int>
    Number of times scenario will be rerun in case of a failure

--retry-after <duration>
    Delay between each retry attempt.

    Duration is represented in human-readable format like `12min5s`.
    Supported suffixes:
    - `nsec`, `ns` — nanoseconds.
    - `usec`, `us` — microseconds.
    - `msec`, `ms` — milliseconds.
    - `seconds`, `second`, `sec`, `s` - seconds.
    - `minutes`, `minute`, `min`, `m` - minutes.

--retry-tag-filter <tagexpr>
    Tag expression to filter retried scenarios
```

`--retry` option is similar to `@retry(<number-of-retries>)`, but applied to all scenarios, that are matched by `--retry-tag-filter` (if not provided, all scenarios will be retried). `--retry-after` is similar to `@retry.after(<delay-after-each-retry>)` in the same manner.




### Interaction between tags and CLI options

- `@retry` will take number of retries from `--retry` and delay from `--retry-after`.
- `@retry(3)` will always retry failed scenarios at most 3 times, even if `--retry` option has a greater value. Delay value will be taken from `--retry-after` option.
- `@retry.after(1s)` will always delay 1 second before next retry. Number of retries value will be taken from `--retry-after` option.
- `@retry(3).after(1s)` will always retry failed scenarios at most 3 times with 1 second delay, ignoring `CLI` options.

> __NOTE__: You can always specify `@retry` without explicit values and run with `--retry=n --retry-after=d --retry-tag-filter=@retry` to overwrite retry options and don't affect any other scenarios.



[`World`]: https://docs.rs/cucumber/latest/cucumber/trait.World.html
[controlling time in tests]: https://docs.rs/tokio/latest/tokio/time/fn.pause.html
[simulation testing]: https://github.com/madsys-dev/madsim
