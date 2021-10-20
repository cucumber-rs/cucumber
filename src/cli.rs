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
#[derive(StructOpt, Debug)]
pub struct Opts<Parser, Runner, Writer>
where
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

// Workaround overwritten doc-comments.
// https://github.com/TeXitoi/structopt/issues/333#issuecomment-712265332
#[cfg_attr(not(doc), allow(missing_docs))]
#[cfg_attr(doc, doc = "Empty CLI options.")]
#[derive(StructOpt, Clone, Copy, Debug)]
pub struct Empty {
    /// This field exists only because [`StructOpt`] derive macro doesn't
    /// support unit structs.
    #[structopt(skip)]
    skipped: (),
}
