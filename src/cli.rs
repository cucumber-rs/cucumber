// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for composing CLI options.
//!
//! The main thing in this module is [`Opts`], which compose all the strongly
//! typed CLI options from [`Parser`], [`Runner`] and [`Writer`], and provide
//! filtering based on [`Regex`] or [tag expressions][1].
//!
//! The idea behind this is that [`Parser`], [`Runner`] and/or [`Writer`] may
//! want to introduce their own CLI options to allow tweaking themselves, but we
//! still do want them combine in a single CLI and avoid any boilerplate burden.
//!
//! If the implementation doesn't need any CLI options, it may just use the
//! prepared [`cli::Empty`] stub.
//!
//! [`cli::Empty`]: self::Empty
//! [`Parser`]: crate::Parser
//! [`Runner`]: crate::Runner
//! [`Writer`]: crate::Writer
//! [1]: https://cucumber.io/docs/cucumber/api#tag-expressions

use gherkin::tagexpr::TagOperation;
use regex::Regex;
use structopt::StructOpt;

// Workaround for overwritten doc-comments.
// https://github.com/TeXitoi/structopt/issues/333#issuecomment-712265332
#[cfg_attr(
    doc,
    doc = r#"
Root CLI (command line interface) of a top-level [`Cucumber`] executor.

It combines all the nested CLIs of [`Parser`], [`Runner`] and [`Writer`],
and may be extended with custom CLI options additionally.

# Example

```rust
# use std::{convert::Infallible, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{cli, WorldInit};
# use futures::FutureExt as _;
# use structopt::StructOpt;
# use tokio::time;
#
# #[derive(Debug, WorldInit)]
# struct MyWorld;
#
# #[async_trait(?Send)]
# impl cucumber::World for MyWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Self::Error> {
#         Ok(Self)
#     }
# }
#
# #[tokio::main(flavor = "current_thread")]
# async fn main() {
#[derive(StructOpt)]
struct CustomOpts {
    /// Additional time to wait in before hook.
    #[structopt(
        long,
        parse(try_from_str = humantime::parse_duration)
    )]
    pre_pause: Option<Duration>,
}

let opts = cli::Opts::<_, _, _, CustomOpts>::from_args();
let pre_pause = opts.custom.pre_pause.unwrap_or_default();

MyWorld::cucumber()
    .before(move |_, _, _, _| time::sleep(pre_pause).boxed_local())
    .with_cli(opts)
    .run_and_exit("tests/features/readme")
    .await;
# }
```

[`Cucumber`]: crate::Cucumber
[`Parser`]: crate::Parser
[`Runner`]: crate::Runner
[`Writer`]: crate::Writer
"#
)]
#[cfg_attr(not(doc), doc = "Run the tests, pet a dog!.")]
#[derive(Debug, Clone, StructOpt)]
#[structopt(name = "cucumber", about = "Run the tests, pet a dog!.")]
pub struct Opts<Parser, Runner, Writer, Custom = Empty>
where
    Parser: StructOpt,
    Runner: StructOpt,
    Writer: StructOpt,
    Custom: StructOpt,
{
    /// Regex to filter scenarios by their name.
    #[structopt(
        short = "n",
        long = "name",
        name = "regex",
        visible_alias = "scenario-name"
    )]
    pub re_filter: Option<Regex>,

    /// Tag expression to filter scenarios by.
    ///
    /// Note: Tags from Feature, Rule and Scenario are merged together on
    /// filtering, so be careful about conflicting tags on different levels.
    #[structopt(
        short = "t",
        long = "tags",
        name = "tagexpr",
        conflicts_with = "regex"
    )]
    pub tags_filter: Option<TagOperation>,

    /// [`Parser`] CLI options.
    ///
    /// [`Parser`]: crate::Parser
    #[structopt(flatten)]
    pub parser: Parser,

    /// [`Runner`] CLI options.
    ///
    /// [`Runner`]: crate::Runner
    #[structopt(flatten)]
    pub runner: Runner,

    /// [`Writer`] CLI options.
    ///
    /// [`Writer`]: crate::Writer
    #[structopt(flatten)]
    pub writer: Writer,

    /// Additional custom CLI options.
    #[structopt(flatten)]
    pub custom: Custom,
}

// Workaround for overwritten doc-comments.
// https://github.com/TeXitoi/structopt/issues/333#issuecomment-712265332
#[cfg_attr(doc, doc = "Empty CLI options.")]
#[cfg_attr(
    not(doc),
    allow(missing_docs, clippy::missing_docs_in_private_items)
)]
#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Empty {
    /// This field exists only because [`StructOpt`] derive macro doesn't
    /// support unit structs.
    #[allow(dead_code)]
    #[structopt(skip)]
    skipped: (),
}

// Workaround for overwritten doc-comments.
// https://github.com/TeXitoi/structopt/issues/333#issuecomment-712265332
#[cfg_attr(
    doc,
    doc = r#"
Composes two [`StructOpt`] derivers together.

# Example

This struct is especially useful, when implementing custom [`Writer`] wrapping
another one:
```rust
# use async_trait::async_trait;
# use cucumber::{cli, event, parser, writer, Event, World, Writer};
# use structopt::StructOpt;
#
struct CustomWriter<Wr>(Wr);

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    custom_option: Option<String>,
}

#[async_trait(?Send)]
impl<W, Wr> Writer<W> for CustomWriter<Wr>
where
    W: World,
    Wr: Writer<W>,
{
    type Cli = cli::Compose<Cli, Wr::Cli>;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        // Some custom logic including `cli.left.custom_option`.
        // ...
        self.0.handle_event(ev, &cli.right).await;
    }
}

// Useful blanket impls:

#[async_trait(?Send)]
impl<'val, W, Wr, Val> writer::Arbitrary<'val, W, Val> for CustomWriter<Wr>
where
    W: World,
    Self: Writer<W>,
    Wr: writer::Arbitrary<'val, W, Val>,
    Val: 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.0.write(val).await;
    }
}

impl<W, Wr> writer::Failure<W> for CustomWriter<Wr>
where
    W: World,
    Self: Writer<W>,
    Wr: writer::Failure<W>,
{
    fn failed_steps(&self) -> usize {
        self.0.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.0.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.0.hook_errors()
    }
}

impl<Wr: writer::Normalized> writer::Normalized for CustomWriter<Wr> {}

impl<Wr: writer::NonTransforming> writer::NonTransforming
    for CustomWriter<Wr>
{}
```

[`Writer`]: crate::Writer
"#
)]
#[cfg_attr(
    not(doc),
    allow(missing_docs, clippy::missing_docs_in_private_items)
)]
#[derive(Debug, StructOpt)]
pub struct Compose<L: StructOpt, R: StructOpt> {
    /// Left [`StructOpt`] deriver.
    #[structopt(flatten)]
    pub left: L,

    /// Right [`StructOpt`] deriver.
    #[structopt(flatten)]
    pub right: R,
}

impl<L: StructOpt, R: StructOpt> Compose<L, R> {
    /// Unpacks this [`Compose`] into the underlying CLIs.
    #[must_use]
    pub fn into_inner(self) -> (L, R) {
        let Compose { left, right } = self;
        (left, right)
    }
}
