TODO



## Cucumber JSON format output

Library provides an ability to output tests result in a [Cucumber JSON format].

Just enable `output-json` library feature in your `Cargo.toml`:
```toml
cucumber = { version = "0.11", features = ["output-json"] }
```

And configure [Cucumber]'s output both to STDOUT and `writer::Json` (with `writer::Tee`):
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
let file = fs::File::create(dbg!(format!("{}/target/schema.json", env!("CARGO_MANIFEST_DIR"))))?;
World::cucumber()
    .with_writer(
        // `Writer`s pipeline is constructed in a reversed order.
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




[Cucumber JSON format]: https://github.com/cucumber/cucumber-json-schema
