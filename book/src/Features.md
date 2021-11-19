Features
========

This chapter contains overview and examples of some [Cucumber] and [Gherkin] features.




## `Rule` keyword

The purpose of the `Rule` keyword is to represent a business rule that should be implemented. It provides additional information for a feature. A `Rule` is used to group together several scenarios that belong to this business rule. A `Rule` should contain one or more scenarios that illustrate the particular rule.

You don't need additional work on the implementation side to support `Rule`s. Let's take final example from [Getting Started](Getting_Started.md) chapter and change the `.feature` file to:

```gherkin
Feature: Animal feature
    
  Rule: Hungry cat becomes satiated
      
    Scenario: If we feed a hungry cat it will no longer be hungry
      Given a hungry cat
      When I feed the cat
      Then the cat is not hungry
    
  Rule: Satiated cat remains the same
      
    Scenario: If we feed a satiated cat it will not become hungry
      Given a satiated cat
      When I feed the cat
      Then the cat is not hungry
```

<script id="asciicast-c8XFsr52VB8tuIJfIaofWAfyh" src="https://asciinema.org/a/c8XFsr52VB8tuIJfIaofWAfyh.js" async data-autoplay="true" data-rows="18"></script>




## `Background` keyword

Occasionally you’ll find yourself repeating the same `Given` steps in all the scenarios of a `Feature`.

Since it's repeated in every scenario, this is an indication that those steps are not essential to describe the scenarios, so they are _incidental details_. You can literally move such `Given` steps to background, by grouping them under a `Background` section.

`Background` allows you to add some context to the `Scenario`s following it. It can contain one or more steps, which are run before each scenario (but after any [`Before` hooks](#before-hook)).

```gherkin
Feature: Animal feature
    
  Background: 
    Given a hungry cat
    
  Rule: Hungry cat becomes satiated
      
    Scenario: If we feed a hungry cat it will no longer be hungry
      When I feed the cat
      Then the cat is not hungry
    
  Rule: Satiated cat remains the same
      
    Background:
      When I feed the cat

    Scenario: If we feed a satiated cat it will not become hungry
      When I feed the cat
      Then the cat is not hungry
```

<script id="asciicast-ZQyfL8gVHD932rskDDESqlsD9" src="https://asciinema.org/a/ZQyfL8gVHD932rskDDESqlsD9.js" async data-autoplay="true" data-rows="18"></script>

`Background` `Step`s indicated by `>` sign in the output by default.

In case `Background` is declared outside any `Rule`, it will be run on any `Scenario`. Otherwise, if `Background` is declared inside `Rule`, it will be run only for `Scenario`s inside this `Rule` and only after top-level `Background` statements, if any.


### Tips for using `Background`

 - Don’t use `Background` to set up complicated states, unless that state is actually something the client needs to know.
 - Keep your `Background` section short.
 - Make your `Background` section vivid, use colorful names, and try to tell a story.
 - Keep your `Scenario`s short, and don’t have too many.

Clearly, example provided above doesn't need `Background` and was done for demonstration purposes only.




## `Scenario Outline` keyword

The `Scenario Outline` keyword can be used to run the same `Scenario` multiple times, with different combinations of values:

```gherkin
Feature: Animal feature

  Scenario Outline: If we feed a hungry animal it will no longer be hungry
    Given a hungry <animal>
    When I feed the <animal>
    Then the <animal> is not hungry

  Examples: 
    | animal |
    | cat    |
    | dog    |
    | 🦀     |
```

And leverage `regex` support to match `Step`s:

```rust
# use std::{convert::Infallible, time::Duration};
# 
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
# 
# #[derive(Debug)]
# struct Cat {
#     pub hungry: bool,
# }
# 
# impl Cat {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
# 
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
# 
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
# 
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Cat { hungry: false },
#         })
#     }
# }
# 
#[given(regex = r"^a (hungry|satiated) (\S+)$")]
async fn hungry_cat(world: &mut AnimalWorld, state: String) {
    sleep(Duration::from_secs(2)).await;

    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when(regex = r"^I feed the (\S+)$")]
async fn feed_cat(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    world.cat.feed();
}

#[then(regex = r"^the (\S+) is not hungry$")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}
# 
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/features/scenario_outline.feature").await;
# }
```

<script id="asciicast-o1s4mSMYkkVBy4WAsG8lhYtT8" src="https://asciinema.org/a/o1s4mSMYkkVBy4WAsG8lhYtT8.js" async data-autoplay="true" data-rows="18"></script>


### Combining `regex` and `FromStr`

At parsing stage, `<templates>` are replaced by value from cells. That means you can parse table cells into any type, that implements [`FromStr`](https://doc.rust-lang.org/stable/std/str/trait.FromStr.html).

```gherkin
Feature: Animal feature

  Scenario Outline: If we feed a hungry animal it will no longer be hungry
    Given a hungry <animal>
    When I feed the <animal> <n> times
    Then the <animal> is not hungry

  Examples: 
    | animal | n |
    | cat    | 2 |
    | dog    | 3 |
    | 🦀     | 4 |
```

```rust
# use std::{convert::Infallible, str::FromStr, time::Duration};
# 
# use async_trait::async_trait;
# use cucumber::{given, then, when, World, WorldInit};
# use tokio::time::sleep;
# 
# #[derive(Debug)]
# struct Cat {
#     pub hungry: bool,
# }
# 
# impl Cat {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
# 
# #[derive(Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Cat,
# }
# 
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
# 
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Cat { hungry: false },
#         })
#     }
# }
# 
enum State {
    Hungry,
    Satiated,
}

impl FromStr for State {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hungry" => Ok(Self::Hungry),
            "satiated" => Ok(Self::Satiated),
            _ => Err("expected hungry or satiated"),
        }
    }
}

#[given(regex = r"^a (\S+) (\S+)$")]
async fn hungry_cat(world: &mut AnimalWorld, state: State) {
    sleep(Duration::from_secs(2)).await;

    match state {
        State::Hungry => world.cat.hungry = true,
        State::Satiated => world.cat.hungry = false,
    }
}

#[when(regex = r"^I feed the (?:\S+) (\d+) times?$")]
async fn feed_cat(world: &mut AnimalWorld, times: usize) {
    sleep(Duration::from_secs(2)).await;

    for _ in 0..times {
        world.cat.feed();
    }
}

#[then(regex = r"^the (\S+) is not hungry$")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}
# 
# #[tokio::main]
# async fn main() {
#     AnimalWorld::run("/tests/features/book/features/scenario_outline_fromstr.feature").await;
# }
```

<script id="asciicast-GeKTIuSZ61Q9Nzv5X4TyrNVVp" src="https://asciinema.org/a/GeKTIuSZ61Q9Nzv5X4TyrNVVp.js" async data-autoplay="true" data-rows="18"></script>




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
cucumber 0.10.0
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
    -t, --tags <tagexpr>               Tag expression to filter scenarios by [aliases: scenario-tags]
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
