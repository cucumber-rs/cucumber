CLI (command-line interface)
============================

[`cucumber`] crate provides several options that can be passed to the command-line.

Use `--help` flag to print out all the available options:
```bash
cargo test --test <test-name> -- --help
```

Default output is:
```
cucumber
Run the tests, pet a dog!

USAGE:
    cucumber [OPTIONS]

OPTIONS:
    -c, --concurrency <int>
            Number of scenarios to run concurrently. If not specified, uses the value configured in
            tests runner, or 64 by default

        --color <auto|always|never>
            Coloring policy for a console output
            
            [default: auto]
            
        --fail-fast
            Run tests until the first failure

    -h, --help
            Print help information

    -i, --input <glob>
            Glob pattern to look for feature files with. By default, looks for `*.feature`s in the
            path configured tests runner

    -n, --name <regex>
            Regex to filter scenarios by their name
            
            [aliases: scenario-name]

    -t, --tags <tagexpr>
            Tag expression to filter scenarios by.
            
            Note: Tags from Feature, Rule and Scenario are merged together on filtering, so be
            careful about conflicting tags on different levels.

    -v
            Verbosity of an output.
            
            `-v` is default verbosity, `-vv` additionally outputs world on failed steps, `-vvv`
            additionally outputs step's doc string (if present).
```

![record](rec/cli.gif)

> __NOTE__: CLI options override any configurations set in the code.




## Customizing

By default, the whole CLI is composed of [`Parser::Cli`], [`Runner::Cli`] and [`Writer::Cli`], provided by the used components. Once a custom [`Parser`], [`Runner`] or [`Writer`] is used, its CLI is automatically emerged into the final CLI.

CLI may be extended even more with arbitrary options, if required. In such case we should combine the final CLI by ourselves and apply it via [`Cucumber::with_cli()`] method.

```rust
# use std::{convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{cli, given, then, when, World, WorldInit};
# use futures::FutureExt as _;
# use tokio::time::sleep;
#
# #[derive(Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     assert!(!world.cat.hungry);
# }
#
#[derive(cli::Args)] // re-export of `clap::Args`
struct CustomOpts {
    /// Additional time to wait in before hook.
    #[clap(
        long,
        parse(try_from_str = humantime::parse_duration)
    )]
    pre_pause: Option<Duration>,
}

#[tokio::main]
async fn main() {
    let opts = cli::Opts::<_, _, _, CustomOpts>::parsed();
    let pre_pause = opts.custom.pre_pause.unwrap_or_default();

    AnimalWorld::cucumber()
        .before(move |_, _, _, _| sleep(pre_pause).boxed_local())
        .with_cli(opts)
        .run_and_exit("/tests/features/book/cli.feature")
        .await;
}
```
![record](rec/cli_custom.gif)

> __NOTE__: For extending CLI options of exising [`Parser`], [`Runner`] or [`Writer`] when wrapping it, consider using [`cli::Compose`].

> __NOTE__: If a custom [`Parser`], [`Runner`] or [`Writer`] implementation doesn't expose any CLI options, then [`cli::Empty`] should be used.




[`cli::Compose`]: https://docs.rs/cucumber/*/cucumber/cli/struct.Compose.html
[`cli::Empty`]: https://docs.rs/cucumber/*/cucumber/cli/struct.Empty.html
[`cucumber`]: https://docs.rs/cucumber
[`Cucumber::with_cli()`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html#method.with_cli
[`Parser`]: architecture/parser.md
[`Parser::Cli`]: https://docs.rs/cucumber/*/cucumber/trait.Parser.html#associatedtype.Cli
[`Runner`]: architecture/runner.md
[`Runner::Cli`]: https://docs.rs/cucumber/*/cucumber/trait.Runner.html#associatedtype.Cli
[`Writer`]: architecture/writer.md
[`Writer::Cli`]: https://docs.rs/cucumber/*/cucumber/trait.Writer.html#associatedtype.Cli
