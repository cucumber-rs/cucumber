Scenario hooks
==============

[Scenario hooks][hook] represent a code running for each [scenario] and not visible in `.feature` files.




## `Before` hook

[`Before` hook] runs before the first step of each scenario, even before [`Background`] ones.

```rust
# extern crate cucumber;
# extern crate futures;
# extern crate tokio;
#
# use std::time::Duration;
#
# use cucumber::World as _;
# use futures::FutureExt as _;
# use tokio::time;
#
# #[derive(Debug, Default, cucumber::World)]
# struct World;
#
# fn main() {
World::cucumber()
    .before(|_feature, _rule, _scenario, _world| {
        time::sleep(Duration::from_millis(300)).boxed_local()
    })
    .run_and_exit("tests/features/book");
# }
```

> __NOTE__: [`Before` hook] is enabled globally for all the executed [scenario]s. No exception is possible.

> __WARNING__: __Think twice before using [`Before` hook]!__
> Whatever happens in a [`Before` hook] is invisible to people reading `.feature`s. You should consider using a [`Background`] keyword as a more explicit alternative, especially if the setup should be readable by non-technical people. Only use a [`Before` hook] for low-level logic such as starting a browser or deleting data from a database.

> __WARNING__: __Only one [`Before` hook] can be registered!__
> Only one [`Before` hook] can be be registered, if multiple `.before` calls are made only the last one will be run.


## `After` hook

[`After` hook] runs after the last [step] of each [scenario], even when that [step] fails or is skipped.

```rust
# extern crate cucumber;
# extern crate futures;
# extern crate tokio;
#
# use std::time::Duration;
#
# use cucumber::World as _;
# use futures::FutureExt as _;
# use tokio::time;
#
# #[derive(Debug, Default, cucumber::World)]
# struct World;
#
# fn main() {
World::cucumber()
    .after(|_feature, _rule, _scenario, _ev, _world| {
        time::sleep(Duration::from_millis(300)).boxed_local()
    })
    .run_and_exit("tests/features/book");
# }
```

> __NOTE__: [`After` hook] is enabled globally for all the executed [scenario]s. No exception is possible.

> __TIP__: [`After` hook] receives an [`event::ScenarioFinished`] as one of its arguments, which indicates why the [scenario] has finished (passed, failed or skipped). This information, for example, may be used to decide whether some external resources (like files) should be cleaned up if the [scenario] passes, or leaved "as is" if it fails, so helping to "freeze" the failure conditions for better investigation.

> __WARNING__: __Only one [`After` hook] can be registered!__
> Only one [`After` hook] can be be registered, if multiple `.after` calls are made only the last one will be run.


[`After` hook]: https://cucumber.io/docs/cucumber/api#after
[`Background`]: background.md
[`Before` hook]: https://cucumber.io/docs/cucumber/api#before
[`event::ScenarioFinished`]: https://docs.rs/cucumber/*/cucumber/event/struct.ScenarioFinished.html
[hook]: https://cucumber.io/docs/cucumber/api#scenario-hooks
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[step]: https://cucumber.io/docs/gherkin/reference#steps
