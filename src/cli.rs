// Copyright (c) 2018-2022  Brendan Molloy <brendan@bbqsrc.net>,
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

use crate::writer::Coloring;

pub use clap::{Args, Parser};

/// Root CLI (command line interface) of a top-level [`Cucumber`] executor.
///
/// It combines all the nested CLIs of [`Parser`], [`Runner`] and [`Writer`],
/// and may be extended with custom CLI options additionally.
///
/// # Example
///
/// ```rust
/// # use std::time::Duration;
/// #
/// # use cucumber::{cli, World};
/// # use futures::FutureExt as _;
/// # use tokio::time;
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// #[derive(clap::Args)] // also re-exported as `cli::Args`
/// struct CustomOpts {
///     /// Additional time to wait in before hook.
///     #[clap(
///         long,
///         parse(try_from_str = humantime::parse_duration)
///     )]
///     pre_pause: Option<Duration>,
/// }
///
/// let opts = cli::Opts::<_, _, _, CustomOpts>::parsed();
/// let pre_pause = opts.custom.pre_pause.unwrap_or_default();
///
/// MyWorld::cucumber()
///     .before(move |_, _, _, _| time::sleep(pre_pause).boxed_local())
///     .with_cli(opts)
///     .run_and_exit("tests/features/readme")
///     .await;
/// # }
/// ```
///
/// [`Cucumber`]: crate::Cucumber
/// [`Parser`]: crate::Parser
/// [`Runner`]: crate::Runner
/// [`Writer`]: crate::Writer
#[derive(Debug, Clone, clap::Parser)]
#[clap(
    name = "cucumber",
    about = "Run the tests, pet a dog!",
    long_about = "Run the tests, pet a dog!"
)]
pub struct Opts<Parser, Runner, Writer, Custom = Empty>
where
    Parser: Args,
    Runner: Args,
    Writer: Args,
    Custom: Args,
{
    /// Regex to filter scenarios by their name.
    #[clap(
        short = 'n',
        long = "name",
        name = "regex",
        visible_alias = "scenario-name",
        global = true
    )]
    pub re_filter: Option<Regex>,

    /// Tag expression to filter scenarios by.
    ///
    /// Note: Tags from Feature, Rule and Scenario are merged together on
    /// filtering, so be careful about conflicting tags on different levels.
    #[clap(
        short = 't',
        long = "tags",
        name = "tagexpr",
        conflicts_with = "regex",
        global = true
    )]
    pub tags_filter: Option<TagOperation>,

    /// [`Parser`] CLI options.
    ///
    /// [`Parser`]: crate::Parser
    #[clap(flatten)]
    pub parser: Parser,

    /// [`Runner`] CLI options.
    ///
    /// [`Runner`]: crate::Runner
    #[clap(flatten)]
    pub runner: Runner,

    /// [`Writer`] CLI options.
    ///
    /// [`Writer`]: crate::Writer
    #[clap(flatten)]
    pub writer: Writer,

    /// Additional custom CLI options.
    #[clap(flatten)]
    pub custom: Custom,
}

impl<Parser, Runner, Writer, Custom> Opts<Parser, Runner, Writer, Custom>
where
    Parser: Args,
    Runner: Args,
    Writer: Args,
    Custom: Args,
{
    /// Shortcut for [`clap::Parser::parse()`], which doesn't require the trait
    /// being imported.
    #[must_use]
    pub fn parsed() -> Self {
        <Self as clap::Parser>::parse()
    }
}

/// Indication whether a [`Writer`] using CLI options supports colored output.
///
/// [`Writer`]: crate::Writer
pub trait Colored {
    /// Returns [`Coloring`] indicating whether a [`Writer`] using CLI options
    /// supports colored output or not.
    ///
    /// [`Writer`]: crate::Writer
    #[must_use]
    fn coloring(&self) -> Coloring {
        Coloring::Never
    }
}

/// Empty CLI options.
#[derive(Args, Clone, Copy, Debug)]
pub struct Empty;

impl Colored for Empty {}

/// Composes two [`clap::Args`] derivers together.
///
/// # Example
///
/// This struct is especially useful, when implementing custom [`Writer`]
/// wrapping another one:
/// ```rust
/// # use async_trait::async_trait;
/// # use cucumber::{cli, event, parser, writer, Event, World, Writer};
/// #
/// struct CustomWriter<Wr>(Wr);
///
/// #[derive(cli::Args)] // re-export of `clap::Args`
/// struct Cli {
///     #[clap(long)]
///     custom_option: Option<String>,
/// }
///
/// #[async_trait(?Send)]
/// impl<W, Wr> Writer<W> for CustomWriter<Wr>
/// where
///     W: World,
///     Wr: Writer<W>,
/// {
///     type Cli = cli::Compose<Cli, Wr::Cli>;
///
///     async fn handle_event(
///         &mut self,
///         ev: parser::Result<Event<event::Cucumber<W>>>,
///         cli: &Self::Cli,
///     ) {
///         // Some custom logic including `cli.left.custom_option`.
///         // ...
///         self.0.handle_event(ev, &cli.right).await;
///     }
/// }
///
/// // Useful blanket impls:
///
/// impl cli::Colored for Cli {}
///
/// #[async_trait(?Send)]
/// impl<'val, W, Wr, Val> writer::Arbitrary<'val, W, Val> for CustomWriter<Wr>
/// where
///     Wr: writer::Arbitrary<'val, W, Val>,
///     Val: 'val,
///     Self: Writer<W>,
/// {
///     async fn write(&mut self, val: Val)
///     where
///         'val: 'async_trait,
///     {
///         self.0.write(val).await;
///     }
/// }
///
/// impl<W, Wr> writer::Stats<W> for CustomWriter<Wr>
/// where
///     Wr: writer::Stats<W>,
///     Self: Writer<W>,
/// {
///     fn passed_steps(&self) -> usize {
///         self.0.failed_steps()
///     }
///
///     fn skipped_steps(&self) -> usize {
///         self.0.failed_steps()
///     }
///
///     fn failed_steps(&self) -> usize {
///         self.0.failed_steps()
///     }
///
///     fn parsing_errors(&self) -> usize {
///         self.0.parsing_errors()
///     }
///
///     fn hook_errors(&self) -> usize {
///         self.0.hook_errors()
///     }
/// }
///
/// impl<Wr: writer::Normalized> writer::Normalized for CustomWriter<Wr> {}
///
/// impl<Wr: writer::NonTransforming> writer::NonTransforming
///     for CustomWriter<Wr>
/// {}
/// ```
///
/// [`Writer`]: crate::Writer
#[derive(Args, Debug)]
pub struct Compose<L: Args, R: Args> {
    /// Left [`clap::Args`] deriver.
    #[clap(flatten)]
    pub left: L,

    /// Right [`clap::Args`] deriver.
    #[clap(flatten)]
    pub right: R,
}

impl<L: Args, R: Args> Compose<L, R> {
    /// Unpacks this [`Compose`] into the underlying CLIs.
    #[allow(clippy::missing_const_for_fn)] // false positive: drop in const
    #[must_use]
    pub fn into_inner(self) -> (L, R) {
        let Compose { left, right } = self;
        (left, right)
    }
}

impl<L, R> Colored for Compose<L, R>
where
    L: Args + Colored,
    R: Args + Colored,
{
    fn coloring(&self) -> Coloring {
        // Basically, founds "maximum" `Coloring` of CLI options.
        match (self.left.coloring(), self.right.coloring()) {
            (Coloring::Always, _) | (_, Coloring::Always) => Coloring::Always,
            (Coloring::Auto, _) | (_, Coloring::Auto) => Coloring::Auto,
            (Coloring::Never, Coloring::Never) => Coloring::Never,
        }
    }
}
