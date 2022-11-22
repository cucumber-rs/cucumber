JUnit XML report
================

[`cucumber`] crate provides an ability to output tests result as a [JUnit XML report].

This requires `output-junit` feature to be enabled in `Cargo.toml`:
```toml
cucumber = { version = "0.16", features = ["output-junit"] }
```

And configuring output to [`writer::JUnit`]:
```rust
# extern crate cucumber;
# extern crate tokio;
#
# use std::{fs, io};
use cucumber::{writer, World as _};

# #[derive(cucumber::World, Debug, Default)]
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
