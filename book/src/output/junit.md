JUnit XML report
================

[`cucumber`] crate provides an ability to output tests result as a [JUnit XML report].

This requires `output-junit` feature to be enabled in `Cargo.toml`:
```toml
cucumber = { version = "0.11", features = ["output-junit"] }
```

And configuring output to [`writer::JUnit`]:
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
let file = fs::File::create(dbg!(format!("{}/target/junit.xml", env!("CARGO_MANIFEST_DIR"))))?;
World::cucumber()
    .with_writer(writer::JUnit::new(file))
    .run("tests/features/book")
    .await;
# Ok(())
# }
```




[`cucumber`]: https://docs.rs/cucumber
[`writer::JUnit`]: https://docs.rs/cucumber/*/cucumber/writer/struct.JUnit.html
[JUnit XML report]: https://llg.cubic.org/docs/junit
