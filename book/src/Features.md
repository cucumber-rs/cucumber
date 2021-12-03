Features
========


















## Spoken languages

The language you choose for [Gherkin] should be the same language your users and domain experts use when they talk about the domain. Translating between two languages should be avoided.

This is why [Gherkin] has been translated to over [70 languages](https://cucumber.io/docs/gherkin/languages).

A `# language:` header on the first line of a `.feature` file tells [Cucumber] which spoken language to use (for example, `# language: fr` for French). If you omit this header, [Cucumber] will default to English (`en`).

```gherkin
# language: no
    
Egenskap: Animal feature
    
  Eksempel: If we feed a hungry cat it will no longer be hungry
    Gitt a hungry cat
    Når I feed the cat
    Så the cat is not hungry
```

<script id="asciicast-DFtCqnpcnXpKbGxtxfedkW0Ga" src="https://asciinema.org/a/DFtCqnpcnXpKbGxtxfedkW0Ga.js" async data-autoplay="true" data-rows="18"></script>

In case most of your `.feature` files aren't written in English and you want to avoid endless `# language:` comments, use [`Cucumber::language()`](https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.language) method to override the default language.




## Scenario hooks


### `Before` hook

`Before` hook runs before the first step of each scenario, even before [`Background` ones](#background-keyword).

```rust
# use std::{convert::Infallible, time::Duration};
# 
# use async_trait::async_trait;
# use cucumber::WorldInit;
# use futures::FutureExt as _;
# use tokio::time;
# 
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
# fn main() {
World::cucumber()
    .before(|_feature, _rule, _scenario, _world| {
        time::sleep(Duration::from_millis(10)).boxed_local()
    })
    .run_and_exit("tests/features/book");
# }
```

> ⚠️ __Think twice before using `Before` hook!__  
> Whatever happens in a `Before` hook is invisible to people reading `.feature`s. You should consider using a [`Background`](#background-keyword) as a more explicit alternative, especially if the setup should be readable by non-technical people. Only use a `Before` hook for low-level logic such as starting a browser or deleting data from a database.


### `After` hook

`After` hook runs after the last step of each `Scenario`, even when that step fails or is skipped.

```rust
# use std::{convert::Infallible, time::Duration};
# 
# use async_trait::async_trait;
# use cucumber::WorldInit;
# use futures::FutureExt as _;
# use tokio::time;
# 
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
# fn main() {
World::cucumber()
    .after(|_feature, _rule, _scenario, _world| {
        time::sleep(Duration::from_millis(10)).boxed_local()
    })
    .run_and_exit("tests/features/book");
# }
```




## CLI options

Library provides several options that can be passed to the command-line.

Use `--help` flag to print out all the available options:
```shell
cargo test --test <test-name> -- --help
```

Default output is:
```
cucumber
Run the tests, pet a dog!

USAGE:
    cucumber [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Increased verbosity of an output: additionally outputs step's doc string (if present)

OPTIONS:
        --color <auto|always|never>    Coloring policy for a console output [default: auto]
    -i, --input <glob>                 Glob pattern to look for feature files with. By default, looks for `*.feature`s
                                       in the path configured tests runner
    -c, --concurrency <int>            Number of scenarios to run concurrently. If not specified, uses the value
                                       configured in tests runner, or 64 by default
    -n, --name <regex>                 Regex to filter scenarios by their name [aliases: scenario-name]
    -t, --tags <tagexpr>               Tag expression to filter scenarios by.
                                       Note: Tags from Feature, Rule and Scenario are merged together on filtering, 
                                       so be careful about conflicting tags on different levels. 
```

Example with [tag expressions](https://cucumber.io/docs/cucumber/api#tag-expressions) for filtering `Scenario`s:
```shell
cargo test --test <test-name> -- --tags='@cat or @dog or @ferris'
```

> Note: CLI overrides any configurations set in the code. 


### Customizing CLI options

CLI options are designed to be composable from the one provided by [`Parser::Cli`](https://docs.rs/cucumber/*/cucumber/trait.Parser.html#associatedtype.Cli), [`Runner::Cli`](https://docs.rs/cucumber/*/cucumber/trait.Runner.html#associatedtype.Cli) and [`Writer::Cli`](https://docs.rs/cucumber/*/cucumber/trait.Writer.html#associatedtype.Cli).

You may also extend CLI options with custom ones, if you have such a need for running your tests. See a [`cli::Opts` example](https://docs.rs/cucumber/*/cucumber/cli/struct.Opts.html#example) for more details.




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




[Cucumber]: https://cucumber.io
[Cucumber JSON format]: https://github.com/cucumber/cucumber-json-schema
[Gherkin]: https://cucumber.io/docs/gherkin
[JUnit XML report]: https://llg.cubic.org/docs/junit
