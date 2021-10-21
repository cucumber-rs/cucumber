// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! CLI options.

use gherkin::tagexpr::TagOperation;
use regex::Regex;
use structopt::StructOpt;

/// Run the tests, pet a dog!.
#[derive(Debug, StructOpt)]
pub struct Opts<Custom, Parser, Runner, Writer>
where
    Custom: StructOpt,
    Parser: StructOpt,
    Runner: StructOpt,
    Writer: StructOpt,
{
    /// Regex to select scenarios from.
    #[structopt(
        short = "n",
        long = "name",
        name = "regex",
        visible_alias = "scenario-name"
    )]
    pub re_filter: Option<Regex>,

    /// Regex to select scenarios from.
    #[structopt(
        short = "t",
        long = "tags",
        name = "tagexpr",
        visible_alias = "scenario-tags",
        conflicts_with = "regex"
    )]
    pub tags_filter: Option<TagOperation>,

    /// Custom CLI options.
    #[structopt(flatten)]
    pub custom: Custom,

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
#[cfg_attr(doc, doc = "Composes two [`StructOpt`] derivers together.")]
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
