Multiple outputs
================

Reporting tests result to multiple outputs simultaneously may be achieved by using [`writer::Tee`].

```rust
# use std::{fs, io};
use cucumber::{writer, World as _, WriterExt as _};

# #[derive(cucumber::World, Debug, Default)]
# struct World;
#
# #[tokio::main]
# async fn main() -> io::Result<()> {
let file = fs::File::create(format!("{}/report.xml", env!("OUT_DIR")))?;
World::cucumber()
    .with_writer(
        // NOTE: `Writer`s pipeline is constructed in a reversed order.
        writer::Basic::stdout() // And output to STDOUT.
            .summarized()       // Simultaneously, add execution summary.
            .tee::<World, _>(writer::JUnit::for_tee(file, 0)) // Then, output to XML file.
            .normalized()       // First, normalize events order.
    )
    .run_and_exit("tests/features/book")
    .await;
# Ok(())
# }
```




## Merging the same `Writer`s

While using [`writer::Tee`] for different `Writer`s is ok most of the time, merging the same `Writer`s isn't so obvious, because they have identical CLI arguments. Because of that you will get runtime panic from [`clap`] for using CLI arguments with the same name:

```rust,should_panic
# use std::{fs, io};
use cucumber::{writer, World as _, WriterExt as _};

# #[derive(cucumber::World, Debug, Default)]
# struct World;
#
# #[tokio::main]
# async fn main() -> io::Result<()> {
    let file = fs::File::create(format!("{}/report.txt", env!("OUT_DIR")))?;
    World::cucumber()
        .with_writer(
            writer::Basic::raw(
                io::stdout(),
                writer::Coloring::Auto,
                writer::Verbosity::Default,
            )
                .tee::<World, _>(writer::Basic::raw(
                    file,
                    writer::Coloring::Never,
                    2,
                ))
                .summarized()
                .normalized(),
        )
        .run_and_exit("tests/features/book")
        .await;
# Ok(())
# }
```

```
thread 'main' panicked at 'Command cucumber: Argument names must be unique, but 'verbose' is in use by more than one argument or group'
```

To avoid this, you should manually construct the [`cli::Opts`] and supply it with [`Cucumber::with_cli()`]. Example below shows 2 [`writer::Basic`] outputting to `stdout` and file:

```rust
# use std::{fs, io};
use cucumber::{cli, writer, World as _, WriterExt as _};

# #[derive(cucumber::World, Debug, Default)]
# struct World;
#
# #[tokio::main]
# async fn main() -> io::Result<()> {
    // Parse CLI arguments for a single `writer::Basic`.
    let cli = cli::Opts::<_, _, writer::basic::Cli>::parsed();
    let cli = cli::Opts {
        re_filter: cli.re_filter,
        tags_filter: cli.tags_filter,
        parser: cli.parser,
        runner: cli.runner,
        // Replicate CLI arguments for every `writer::Basic`. 
        writer: cli::Compose {
            left: cli.writer.clone(),
            right: cli.writer,
        },
        custom: cli.custom,
    };
    
    let file = fs::File::create(format!("{}/report.txt", env!("OUT_DIR")))?;
    World::cucumber()
        .with_writer(
            writer::Basic::raw(
                io::stdout(),
                writer::Coloring::Auto,
                writer::Verbosity::Default,
            )
                .tee::<World, _>(writer::Basic::raw(
                    file,
                    writer::Coloring::Never,
                    2,
                ))
                .summarized()
                .normalized(),
        )
        .with_cli(cli) // Supply parsed `cli::Opts`
        .run_and_exit("tests/features/book")
        .await;
# Ok(())
# }
```




[`cli::Opts`]: https://docs.rs/cucumber/*/cucumber/cli/struct.Opts.html
[`writer::Basic`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Basic.html
[`writer::Tee`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Tee.html
[`Cucumber::with_cli()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.with_cli

[`clap`]: https://docs.rs/clap/
