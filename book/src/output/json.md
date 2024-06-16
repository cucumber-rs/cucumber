Cucumber JSON format
====================

[`cucumber`] crate provides an ability to output tests result in a [Cucumber JSON format].

This requires `output-json` feature to be enabled in `Cargo.toml`:
```toml
cucumber = { version = "0.20", features = ["output-json"] }
```

And configuring output to [`writer::Json`]:
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
let file = fs::File::create(format!("{}/report.json", env!("OUT_DIR")))?;
World::cucumber()
    .with_writer(writer::Json::new(file))
    .run("tests/features/book")
    .await;
# Ok(())
# }
```




[`cucumber`]: https://docs.rs/cucumber
[`writer::Json`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Json.html
[Cucumber JSON format]: https://github.com/cucumber/cucumber-json-schema
