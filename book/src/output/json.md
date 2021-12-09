Cucumber JSON format
====================

[`cucumber`] crate provides an ability to output tests result in a [Cucumber JSON format].

This requires `output-json` feature to be enabled in `Cargo.toml`:
```toml
cucumber = { version = "0.11", features = ["output-json"] }
```

And configuring output to [`writer::Json`]:
```rust
# use std::{convert::Infallible, fs, io};
# 
# use async_trait::async_trait;
# use cucumber::WorldInit;
use cucumber::writer;

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
    .with_writer(writer::Json::new(file))
    .run_and_exit("tests/features/book")
    .await;
# Ok(())
# }
```




[`cucumber`]: https://docs.rs/cucumber
[`writer::Json`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Json.html
[Cucumber JSON format]: https://github.com/cucumber/cucumber-json-schema
