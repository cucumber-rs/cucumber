// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Main CLI options structure for the cucumber framework.
//!
//! This module contains the primary [`Opts`] struct that combines all CLI
//! options from [`Parser`], [`Runner`], and [`Writer`] components, along with
//! filtering capabilities based on regex patterns or tag expressions.

use clap::{Args, Parser};
use gherkin::tagexpr::TagOperation;
use regex::Regex;

use super::compose::Empty;

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
///     #[arg(
///         long,
///         value_parser = humantime::parse_duration,
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
#[derive(Clone, Debug, Default, Parser)]
#[command(
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
    #[arg(
        id = "name",
        long = "name",
        short = 'n',
        value_name = "regex",
        visible_alias = "scenario-name",
        global = true
    )]
    pub re_filter: Option<Regex>,

    /// Tag expression to filter scenarios by.
    ///
    /// Note: Tags from Feature, Rule and Scenario are merged together on
    /// filtering, so be careful about conflicting tags on different levels.
    #[arg(
        id = "tags",
        long = "tags",
        short = 't',
        value_name = "tagexpr",
        conflicts_with = "name",
        global = true
    )]
    pub tags_filter: Option<TagOperation>,

    /// [`Parser`] CLI options.
    ///
    /// [`Parser`]: crate::Parser
    #[command(flatten)]
    pub parser: Parser,

    /// [`Runner`] CLI options.
    ///
    /// [`Runner`]: crate::Runner
    #[command(flatten)]
    pub runner: Runner,

    /// [`Writer`] CLI options.
    ///
    /// [`Writer`]: crate::Writer
    #[command(flatten)]
    pub writer: Writer,

    /// Additional custom CLI options.
    #[command(flatten)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::Coloring;

    #[derive(Debug, Default, Clone, clap::Args)]
    struct MockParser;

    #[derive(Debug, Default, Clone, clap::Args)]
    struct MockRunner;

    #[derive(Debug, Default, Clone, clap::Args)]
    struct MockWriter;

    impl crate::cli::Colored for MockWriter {
        fn coloring(&self) -> Coloring {
            Coloring::Auto
        }
    }

    #[derive(Debug, Default, Clone, clap::Args)]
    struct CustomOpts {
        #[arg(long)]
        custom_flag: bool,
    }

    #[test]
    fn test_opts_parsing() {
        let args = vec!["cucumber", "--name", "test.*", "--custom-flag"];
        let opts = Opts::<MockParser, MockRunner, MockWriter, CustomOpts>::try_parse_from(args).unwrap();
        
        assert!(opts.re_filter.is_some());
        assert_eq!(opts.re_filter.unwrap().as_str(), "test.*");
        assert!(opts.custom.custom_flag);
    }

    #[test]
    fn test_opts_with_tags_filter() {
        let args = vec!["cucumber", "--tags", "@smoke and not @slow"];
        let opts = Opts::<MockParser, MockRunner, MockWriter, Empty>::try_parse_from(args).unwrap();
        
        assert!(opts.tags_filter.is_some());
        assert!(opts.re_filter.is_none());
    }

    #[test]
    fn test_conflicting_filters() {
        let args = vec!["cucumber", "--name", "test.*", "--tags", "@smoke"];
        let result = Opts::<MockParser, MockRunner, MockWriter, Empty>::try_parse_from(args);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_opts_parsed_shortcut() {
        // This would normally parse from command line args, but we can test the method exists
        let args = vec!["cucumber"];
        let result = Opts::<Empty, Empty, Empty, Empty>::try_parse_from(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_opts_default_values() {
        let opts = Opts::<Empty, Empty, Empty, Empty>::default();
        assert!(opts.re_filter.is_none());
        assert!(opts.tags_filter.is_none());
    }

    #[test]
    fn test_opts_regex_validation() {
        let args = vec!["cucumber", "--name", "[invalid"];
        let result = Opts::<Empty, Empty, Empty, Empty>::try_parse_from(args);
        assert!(result.is_err());
    }
}