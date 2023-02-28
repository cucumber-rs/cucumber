`tracing` integration
=====================

[`Cucumber::init_tracing()`] (enabled by `tracing` feature in `Cargo.toml`) initializes global [`tracing::Subscriber`] that intercepts all [`tracing` events][1] and transforms them into [`event::Scenario::Log`]s. Each [`Writer`] can handle those events in its own way. [`writer::Basic`] for example, emits all `Scenario` logs, only when `Scenario` itself is outputted:

```rust
# extern crate cucumber;
# extern crate tokio;
# extern crate tracing;
#
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use cucumber::{given, then, when, World as _};
use tokio::time;

#[derive(cucumber::World, Debug, Default)]
struct World;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
async fn sleep(_: &mut World, secs: u64) {
    static ID: AtomicUsize = AtomicUsize::new(0);

    let id = ID.fetch_add(1, Ordering::Relaxed);

    tracing::info!("before {secs}s sleep: {id}");
    time::sleep(Duration::from_secs(secs)).await;
    tracing::info!("after {secs}s sleep: {id}");
}

#[tokio::main]
async fn main() {
    World::cucumber()
        .init_tracing()
        .run("tests/features/wait")
        .await;
}
```

![record](../rec/tracing_basic_writer.gif)

---

Tracking which `Scenario` log is emitted in is done with [`tracing::Span`]s: each `Scenario` is executed in its own [`tracing::Span`]. In case [`tracing` event][1] is emitted outside the [`tracing::Span`] of a `Scenario`, it will be propagated to every running `Scenario`:

```rust
# extern crate cucumber;
# extern crate tokio;
# extern crate tracing;
#
# use std::{
#     sync::atomic::{AtomicUsize, Ordering},
#     time::Duration,
# };
# 
# use cucumber::{given, then, when, World as _};
# use tokio::time;
# 
# #[derive(cucumber::World, Debug, Default)]
# struct World;
# 
# #[given(regex = r"(\d+) secs?")]
# #[when(regex = r"(\d+) secs?")]
# #[then(regex = r"(\d+) secs?")]
# async fn sleep(_: &mut World, secs: u64) {
#     static ID: AtomicUsize = AtomicUsize::new(0);
# 
#     let id = ID.fetch_add(1, Ordering::Relaxed);
# 
#     tracing::info!("before {secs}s sleep: {id}");
#     time::sleep(Duration::from_secs(secs)).await;
#     tracing::info!("after {secs}s sleep: {id}");
# }
# 
#[tokio::main]
async fn main() {
    // Background task outside of any `Scenario`.
    tokio::spawn(async {
        let mut id = 0;
        loop {
            time::sleep(Duration::from_secs(3)).await;
            tracing::info!("Background: {id}");
            id += 1;
        }
    });


    World::cucumber()
        .init_tracing()
        .run("tests/features/wait")
        .await;
}
```

![record](../rec/tracing_outside_span.gif)

`Background: 2`/`3`/`4` is shown in multiple `Scenario`s. 




[`Cucumber::init_tracing()`]: https://docs.rs/cucumber/latest/cucumber/struct.Cucumber.html#method.init_tracing
[`event::Scenario::Log`]: https://docs.rs/cucumber/latest/cucumber/event/enum.Scenario.html#variant.Log
[`tracing::Span`]: https://docs.rs/tracing/latest/tracing/struct.Span.html
[`tracing::Subscriber`]: https://docs.rs/tracing/latest/tracing/trait.Subscriber.html
[`Writer`]: https://docs.rs/cucumber/latest/cucumber/writer/trait.Writer.html
[`writer::Basic`]: https://docs.rs/cucumber/latest/cucumber/writer/struct.Basic.html

[1]: https://docs.rs/tracing/latest/tracing/index.html#events
