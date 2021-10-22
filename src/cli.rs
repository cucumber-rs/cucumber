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
//! Main part of this module is [`Opts`], which composes all strongly-typed
//! `CLI` options from [`Parser`], [`Runner`] and [`Writer`] and adds filtering
//! based on [`Regex`] or [`Tag Expressions`][1].
//!
//! [1]: https://cucumber.io/docs/cucumber/api/#tag-expressions
//! [`Parser`]: crate::Parser
//! [`Runner`]: crate::Runner
//! [`Writer`]: crate::Writer

use gherkin::tagexpr::TagOperation;
use regex::Regex;
use structopt::StructOpt;

/// Run the tests, pet a dog!.
#[derive(Debug, StructOpt)]
pub struct Opts<Parser, Runner, Writer, Custom = Empty>
where
    Custom: StructOpt,
    Parser: StructOpt,
    Runner: StructOpt,
    Writer: StructOpt,
{
    /// Regex to filter scenarios with.
    #[structopt(
        short = "n",
        long = "name",
        name = "regex",
        visible_alias = "scenario-name"
    )]
    pub re_filter: Option<Regex>,

    /// Tag expression to filter scenarios with.
    #[structopt(
        short = "t",
        long = "tags",
        name = "tagexpr",
        visible_alias = "scenario-tags",
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

    /// Custom CLI options.
    #[structopt(flatten)]
    pub custom: Custom,
}

// Workaround for overwritten doc-comments.
// https://github.com/TeXitoi/structopt/issues/333#issuecomment-712265332
#[cfg_attr(
    not(doc),
    allow(missing_docs, clippy::missing_docs_in_private_items)
)]
#[cfg_attr(doc, doc = "Empty CLI options.")]
#[derive(Clone, Copy, Debug, StructOpt)]
pub struct Empty {
    /// This field exists only because [`StructOpt`] derive macro doesn't
    /// support unit structs.
    #[structopt(skip)]
    skipped: (),
}

// Workaround for overwritten doc-comments.
// https://github.com/TeXitoi/structopt/issues/333#issuecomment-712265332
#[cfg_attr(
    not(doc),
    allow(missing_docs, clippy::missing_docs_in_private_items)
)]
#[cfg_attr(
    doc,
    doc = r#"
Composes two [`StructOpt`] derivers together.

# Example

This struct is especially useful, when implementing custom [`Writer`], which
wraps another [`Writer`].

```rust
# use async_trait::async_trait;
# use cucumber::{
#     cli, event, parser, ArbitraryWriter, FailureWriter, World, Writer,
# };
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
        ev: parser::Result<event::Cucumber<W>>,
        cli: &Self::Cli,
    ) {
        // Some custom logic including `cli.left.custom_option`.

        self.0.handle_event(ev, &cli.right).await;
    }
}

// useful blanket impls

#[async_trait(?Send)]
impl<'val, W, Wr, Val> ArbitraryWriter<'val, W, Val> for CustomWriter<Wr>
where
    W: World,
    Self: Writer<W>,
    Wr: ArbitraryWriter<'val, W, Val>,
    Val: 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.0.write(val).await;
    }
}

impl<W, Wr> FailureWriter<W> for CustomWriter<Wr>
where
    W: World,
    Self: Writer<W>,
    Wr: FailureWriter<W>,
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
```

[`Writer`]: crate::Writer"#
)]
#[derive(Debug, StructOpt)]
pub struct Compose<L, R>
where
    L: StructOpt,
    R: StructOpt,
{
    /// [`StructOpt`] deriver.
    #[structopt(flatten)]
    pub left: L,

    /// [`StructOpt`] deriver.
    #[structopt(flatten)]
    pub right: R,
}

impl<L, R> Compose<L, R>
where
    L: StructOpt,
    R: StructOpt,
{
    /// Unpacks [`Compose`] into underlying `CLI`s.
    pub fn into_inner(self) -> (L, R) {
        let Compose { left, right } = self;
        (left, right)
    }
}
