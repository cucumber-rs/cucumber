TODO


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

