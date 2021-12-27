Multiple outputs
================

Reporting tests result to multiple outputs simultaneously may be achieved by using [`writer::Tee`].

```rust
# use std::{convert::Infallible, fs, io};
# 
# use async_trait::async_trait;
# use cucumber::WorldInit;
use cucumber::{writer, WriterExt as _};

# #[derive(Debug, WorldInit)]
# struct World;
# 
# #[async_trait(?Send)]
# impl cucumber::World for World {
#     type Error = Infallible;
# 
#     async fn new() -> Result<Self, Self::Error> {
#         Ok(World)
#     }
# }
#
# #[tokio::main]
# async fn main() -> io::Result<()> {
let file = fs::File::create(dbg!(format!("{}/target/report.json", env!("CARGO_MANIFEST_DIR"))))?;
World::cucumber()
    .with_writer(
        // NOTE: `Writer`s pipeline is constructed in a reversed order.
        writer::Basic::stdout() // And output to STDOUT.
            .summarized()       // Simultaneously, add execution summary.
            .tee::<World, _>(writer::Json::for_tee(file)) // Then, output to JSON file.
            .normalized()       // First, normalize events order.
    )
    .run_and_exit("tests/features/book")
    .await;
# Ok(())
# }
```




[`writer::Tee`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Tee.html
