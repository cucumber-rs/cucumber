TODO


## JUnit XML report

Library provides an ability to output tests result in as [JUnit XML report].

Just enable `output-junit` library feature in your `Cargo.toml`:
```toml
cucumber = { version = "0.11", features = ["output-junit"] }
```

And configure [Cucumber]'s output to `writer::JUnit`:
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




[JUnit XML report]: https://llg.cubic.org/docs/junit
