IntelliJ Rust (`libtest`) integration
=====================================

[`writer::Libtest`] (enabled by `libtest` feature in `Cargo.toml`) allows [IntelliJ Rust] plugin to interpret output of [`cucumber`] tests similar to unit tests. To use it, just add [Cargo configuration][1] (current example uses `cargo test --test wait --features libtest` command) or run it via [Cargo command][2]. This automatically adds `--format=json` CLI option, which makes the [`cucumber`]'s output IDE-compatible.

Example below is set up to output with the default [`writer::Basic`] if there is no `--format=json` option, or with [`writer::Libtest`] otherwise.
```toml
cucumber = { version = "0.20", features = ["libtest"] }
```
```rust
# extern crate cucumber;
# extern crate tokio;
#
use cucumber::{writer, World as _};

# #[derive(cucumber::World, Debug, Default)]
# struct World;
#
# #[tokio::main]
# async fn main() {
World::cucumber()
    .with_writer(writer::Libtest::or_basic())
    .run("tests/features/book")
    .await;
# }
```

![record](../rec/output_intellij.gif)

> __NOTE__: There are currently 2 caveats with [IntelliJ Rust] integration:
> 1. Because of [output interpretation issue][3], current timing reports for individual tests are accurate only for serial tests (or for all in case `--concurrency=1` CLI option is used);
> 2. Although debugger works, test window may select `Step` that didn't trigger the breakpoint. To fix this, use `--concurrency=1` CLI option.

> __TIP__: In the multi-crate [Cargo workspace], to support jump-to-definition in the reported paths ([step] or its matcher definition) correctly, consider to define [`CARGO_WORKSPACE_DIR` environment variable in the `.cargo/config.toml` file][4]:
> ```toml
> [env]
> CARGO_WORKSPACE_DIR = { value = "", relative = true }
> ```




## `libtest` support

Only a small subset of [`libtest`] harness is supported to integrate with other tools:
- Only [`--format=json`][5] output ([`JUnit` support is done separately](junit.md));
- [`--report-time`][6] option;
- [`--show-output`][7] option.




[`cucumber`]: https://docs.rs/cucumber
[`libtest`]: https://doc.rust-lang.org/rustc/tests/index.html
[`writer::Basic`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Basic.html
[`writer::Libtest`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Libtest.html
[Cargo workspace]: https://doc.rust-lang.org/cargo/reference/workspaces.html
[IntelliJ Rust]: https://www.jetbrains.com/rust
[step]: https://cucumber.io/docs/gherkin/reference#steps

[1]: https://plugins.jetbrains.com/plugin/8182-rust/docs/rust-testing.html
[2]: https://plugins.jetbrains.com/plugin/8182-rust/docs/cargo-command-configuration.html
[3]: https://github.com/intellij-rust/intellij-rust/issues/9041
[4]: https://github.com/rust-lang/cargo/issues/3946#issuecomment-973132993
[5]: https://doc.rust-lang.org/rustc/tests/index.html#--format-format
[6]: https://doc.rust-lang.org/rustc/tests/index.html#--report-time
[7]: https://doc.rust-lang.org/rustc/tests/index.html#--show-output
