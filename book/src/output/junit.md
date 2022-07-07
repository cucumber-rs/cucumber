JUnit XML report
================

[`cucumber`] crate provides an ability to output tests result as a [JUnit XML report].

This requires `output-junit` feature to be enabled in `Cargo.toml`:
```toml
cucumber = { version = "0.13", features = ["output-junit"] }
```

And configuring output to [`writer::JUnit`]:
```rust
# use std::{fs, io};
# 
# use cucumber::WorldInit as _;
use cucumber::writer;

# #[derive(Debug, Default, cucumber::World)]
# struct World;
#
# #[tokio::main]
# async fn main() -> io::Result<()> {
let file = fs::File::create(dbg!(format!("{}/junit.xml", env!("OUT_DIR"))))?;
World::cucumber()
    .with_writer(writer::JUnit::new(file, 0))
    .run("tests/features/book")
    .await;
# Ok(())
# }
```




[`cucumber`]: https://docs.rs/cucumber
[`writer::JUnit`]: https://docs.rs/cucumber/*/cucumber/writer/struct.JUnit.html
[JUnit XML report]: https://llg.cubic.org/docs/junit
