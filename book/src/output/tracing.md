`tracing` integration
=====================

[`Cucumber::init_tracing()`] (enabled by `tracing` feature in `Cargo.toml`) initializes global [`tracing::Subscriber`] that intercepts all [`tracing` events][1] and transforms them into [`event::Scenario::Log`]s. Each [`Writer`] can handle those events in its own way. [`writer::Basic`] for example, emits all logs, only when `Scenario` itself is outputted:

```rust
# extern crate cucumber;
# extern crate tokio;
# extern crate tracing;
#
use std::time::Duration;

use cucumber::{given, then, when, World as _};
use tokio::time;

#[derive(cucumber::World, Debug, Default)]
struct World;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
async fn sleep(_: &mut World, secs: u64) {
    tracing::info!("before {secs}s sleep");
    time::sleep(Duration::from_secs(secs)).await;
    tracing::info!("after {secs}s sleep");
}

#[tokio::main]
async fn main() {
    World::cucumber()
        .init_tracing()
        .run("tests/features/wait")
        .await;
}
```

[`Cucumber::init_tracing()`]: https://docs.rs/cucumber/latest/cucumber/struct.Cucumber.html#method.init_tracing
[`event::Scenario::Log`]: https://docs.rs/cucumber/latest/cucumber/event/enum.Scenario.html#variant.Log
[`tracing::Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
[`Writer`]: https://docs.rs/cucumber/latest/cucumber/writer/trait.Writer.html
[`writer::Basic`]: https://docs.rs/cucumber/latest/cucumber/writer/struct.Basic.html

[1]: https://docs.rs/tracing/latest/tracing/index.html#events
