// Copyright (c) 2018-2026  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Top-level [Cucumber] executor.
//!
//! [Cucumber]: https://cucumber.io

use std::{borrow::Cow, marker::PhantomData, mem, path::Path, time::Duration};

use derive_more::with_trait::Debug;
use futures::{StreamExt as _, future::LocalBoxFuture};
use gherkin::tagexpr::TagOperation;
use regex::Regex;

use crate::{
    Event, Parser, Runner, ScenarioType, Step, World, Writer, WriterExt as _,
    cli, event, parser,
    runner::{self, basic::RetryOptions},
    step,
    tag::Ext as _,
    writer,
};

/// Top-level [Cucumber] executor.
///
/// Most of the time you don't need to work with it directly, just use
/// [`World::run()`] or [`World::cucumber()`] on your [`World`] deriver to get
/// [Cucumber] up and running.
///
/// Otherwise, use [`Cucumber::new()`] to get the default [Cucumber] executor,
/// provide [`Step`]s with [`World::collection()`] or by hand with
/// [`Cucumber::given()`], [`Cucumber::when()`] and [`Cucumber::then()`].
///
/// In case you want a custom [`Parser`], [`Runner`] or [`Writer`], or some
/// other finer control, use [`Cucumber::custom()`] or
/// [`Cucumber::with_parser()`], [`Cucumber::with_runner()`] and
/// [`Cucumber::with_writer()`] to construct your dream [Cucumber] executor!
///
/// [Cucumber]: https://cucumber.io
#[derive(Debug)]
pub struct Cucumber<W, P, I, R, Wr, Cli = cli::Empty>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W>,
    Cli: clap::Args,
{
    /// [`Parser`] sourcing [`Feature`]s for execution.
    ///
    /// [`Feature`]: gherkin::Feature
    parser: P,

    /// [`Runner`] executing [`Scenario`]s and producing [`event`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) runner: R,

    /// [`Writer`] outputting [`event`]s to some output.
    writer: Wr,

    /// CLI options this [`Cucumber`] has been run with.
    ///
    /// If empty, then will be parsed from a command line.
    cli: Option<cli::Opts<P::Cli, R::Cli, Wr::Cli, Cli>>,

    /// Explicit composable selection; absence preserves CLI override behavior.
    filter_options: Option<crate::filter::FilterOptions>,

    /// Type of the [`World`] this [`Cucumber`] run on.
    #[debug(ignore)]
    _world: PhantomData<W>,

    /// Type of the input consumed by [`Cucumber::parser`].
    #[debug(ignore)]
    _parser_input: PhantomData<I>,
}

impl<W, P, I, R, Wr, Cli> Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W>,
    Cli: clap::Args,
{
    /// Creates a custom [`Cucumber`] executor with the provided [`Parser`],
    /// [`Runner`] and [`Writer`].
    #[must_use]
    pub const fn custom(parser: P, runner: R, writer: Wr) -> Self {
        Self {
            parser,
            runner,
            writer,
            cli: None,
            filter_options: None,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Enables composable name, tag and caller-predicate selection.
    ///
    /// Typed names and an existing CLI name form one OR group. All typed tag
    /// expressions, an existing CLI tag expression, and the `filter_run`
    /// predicate must also match. Empty options still opt into composition.
    /// Options survive cloning and parser, runner or writer replacement.
    /// The public CLI layout and its existing name/tag conflict are unchanged.
    #[must_use]
    pub fn with_filter_options(
        mut self,
        options: crate::filter::FilterOptions,
    ) -> Self {
        self.filter_options = Some(options);
        self
    }

    /// Replaces [`Parser`].
    #[must_use]
    pub fn with_parser<NewP, NewI>(
        self,
        parser: NewP,
    ) -> Cucumber<W, NewP, NewI, R, Wr, Cli>
    where
        NewP: Parser<NewI>,
    {
        let Self { runner, writer, filter_options, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: None,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Runner`].
    #[must_use]
    pub fn with_runner<NewR>(
        self,
        runner: NewR,
    ) -> Cucumber<W, P, I, NewR, Wr, Cli>
    where
        NewR: Runner<W>,
    {
        let Self { parser, writer, filter_options, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: None,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Writer`].
    #[must_use]
    pub fn with_writer<NewWr>(
        self,
        writer: NewWr,
    ) -> Cucumber<W, P, I, R, NewWr, Cli>
    where
        NewWr: Writer<W>,
    {
        let Self { parser, runner, filter_options, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: None,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Re-outputs [`Skipped`] steps for easier navigation.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::run()`]:
    /// <script
    ///     id="asciicast-0d92qlT8Mbc4WXyvRbHJmjsqN"
    ///     src="https://asciinema.org/a/0d92qlT8Mbc4WXyvRbHJmjsqN.js"
    ///     async data-autoplay="true" data-rows="17">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all the [`Skipped`] steps at the end:
    /// ```rust
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .repeat_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-ox14HynkBIw8atpfhyfvKrsO3"
    ///     src="https://asciinema.org/a/ox14HynkBIw8atpfhyfvKrsO3.js"
    ///     async data-autoplay="true" data-rows="19">
    /// </script>
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    #[must_use]
    pub fn repeat_skipped(
        self,
    ) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr>, Cli>
    where
        Wr: writer::NonTransforming,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_skipped(),
            cli: self.cli,
            filter_options: self.filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Re-outputs [`Failed`] steps for easier navigation.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::fail_on_skipped()`]:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-UcipuopO6IFEsIDty6vaJlCH9"
    ///     src="https://asciinema.org/a/UcipuopO6IFEsIDty6vaJlCH9.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all the [`Failed`] steps at the end:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .repeat_failed()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-ofOljvyEMb41OTLhE081QKv68"
    ///     src="https://asciinema.org/a/ofOljvyEMb41OTLhE081QKv68.js"
    ///     async data-autoplay="true" data-rows="24">
    /// </script>
    ///
    /// [`Failed`]: event::Step::Failed
    #[must_use]
    pub fn repeat_failed(
        self,
    ) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr>, Cli>
    where
        Wr: writer::NonTransforming,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_failed(),
            cli: self.cli,
            filter_options: self.filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Re-outputs steps by the given `filter` predicate.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::fail_on_skipped()`]:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// # use futures::FutureExt as _;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-UcipuopO6IFEsIDty6vaJlCH9"
    ///     src="https://asciinema.org/a/UcipuopO6IFEsIDty6vaJlCH9.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all the [`Failed`] steps ta the end by
    /// providing a custom `filter` predicate:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .repeat_if(|ev| {
    ///         use cucumber::event::{
    ///             Cucumber, Feature, RetryableScenario, Rule, Scenario, Step,
    ///         };
    ///
    ///         matches!(
    ///             ev.as_deref(),
    ///             Ok(Cucumber::Feature(
    ///                 _,
    ///                 Feature::Rule(
    ///                     _,
    ///                     Rule::Scenario(
    ///                         _,
    ///                         RetryableScenario {
    ///                             event: Scenario::Step(_, Step::Failed(..))
    ///                                 | Scenario::Background(
    ///                                     _,
    ///                                     Step::Failed(_, _, _, _),
    ///                                 ),
    ///                             retries: _
    ///                         }
    ///                     )
    ///                 ) | Feature::Scenario(
    ///                     _,
    ///                     RetryableScenario {
    ///                         event: Scenario::Step(_, Step::Failed(..))
    ///                             | Scenario::Background(_, Step::Failed(..)),
    ///                         retries: _
    ///                     }
    ///                 )
    ///             )) | Err(_)
    ///         )
    ///     })
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-ofOljvyEMb41OTLhE081QKv68"
    ///     src="https://asciinema.org/a/ofOljvyEMb41OTLhE081QKv68.js"
    ///     async data-autoplay="true" data-rows="24">
    /// </script>
    ///
    /// [`Failed`]: event::Step::Failed
    #[must_use]
    pub fn repeat_if<F>(
        self,
        filter: F,
    ) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr, F>, Cli>
    where
        F: Fn(&parser::Result<Event<event::Cucumber<W>>>) -> bool,
        Wr: writer::NonTransforming,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_if(filter),
            cli: self.cli,
            filter_options: self.filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Consider [`Skipped`] [`Background`] or regular [`Step`]s as [`Failed`]
    /// if their [`Scenario`] isn't marked with `@allow.skipped` tag.
    ///
    /// It's useful option for ensuring that all the steps were covered.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::run()`]:
    /// <script
    ///     id="asciicast-0d92qlT8Mbc4WXyvRbHJmjsqN"
    ///     src="https://asciinema.org/a/0d92qlT8Mbc4WXyvRbHJmjsqN.js"
    ///     async data-autoplay="true" data-rows="17">
    /// </script>
    ///
    /// To fail all the [`Skipped`] steps setup [`Cucumber`] like this:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-IHLxMEgku9BtBVkR4k2DtOjMd"
    ///     src="https://asciinema.org/a/IHLxMEgku9BtBVkR4k2DtOjMd.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// To intentionally suppress some [`Skipped`] steps failing, use the
    /// `@allow.skipped` tag:
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   @allow.skipped
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    ///
    /// [`Background`]: gherkin::Background
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn fail_on_skipped(
        self,
    ) -> Cucumber<W, P, I, R, writer::FailOnSkipped<Wr>, Cli> {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.fail_on_skipped(),
            cli: self.cli,
            filter_options: self.filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Consider [`Skipped`] [`Background`] or regular [`Step`]s as [`Failed`]
    /// if the given `filter` predicate returns `true`.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::run()`]:
    /// <script
    ///     id="asciicast-0d92qlT8Mbc4WXyvRbHJmjsqN"
    ///     src="https://asciinema.org/a/0d92qlT8Mbc4WXyvRbHJmjsqN.js"
    ///     async data-autoplay="true" data-rows="17">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to fail on all [`Skipped`] steps, but the ones
    /// marked with a `@dog` tag:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped_with(|_, _, s| !s.tags.iter().any(|t| t == "dog"))
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    /// <script
    ///     id="asciicast-IHLxMEgku9BtBVkR4k2DtOjMd"
    ///     src="https://asciinema.org/a/IHLxMEgku9BtBVkR4k2DtOjMd.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// And to avoid failing, use the `@dog` tag:
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   @dog
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    ///
    /// [`Background`]: gherkin::Background
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn fail_on_skipped_with<Filter>(
        self,
        filter: Filter,
    ) -> Cucumber<W, P, I, R, writer::FailOnSkipped<Wr, Filter>, Cli>
    where
        Filter: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> bool,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.fail_on_skipped_with(filter),
            cli: self.cli,
            filter_options: self.filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

impl<W, P, I, R, Wr, Cli> Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W> + writer::Normalized,
    Cli: clap::Args,
{
    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// [`Feature`]: gherkin::Feature
    pub async fn run(self, input: I) -> Wr {
        self.filter_run(input, |_, _, _| true).await
    }

    /// Consumes already parsed [`cli::Opts`].
    ///
    /// This method allows to pre-parse [`cli::Opts`] for custom needs before
    /// using them inside [`Cucumber`].
    ///
    /// Also, any additional custom CLI options may be specified as a
    /// [`clap::Args`] deriving type, used as the last type parameter of
    /// [`cli::Opts`].
    ///
    /// > ⚠️ __WARNING__: Any CLI options of [`Parser`], [`Runner`], [`Writer`]
    /// >                 or custom ones should not overlap, otherwise
    /// >                 [`cli::Opts`] will fail to parse on startup.
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
    /// #[derive(clap::Args)]
    /// struct CustomCli {
    ///     /// Additional time to wait in a before hook.
    ///     #[arg(
    ///         long,
    ///         value_parser = humantime::parse_duration,
    ///     )]
    ///     before_time: Option<Duration>,
    /// }
    ///
    /// let cli = cli::Opts::<_, _, _, CustomCli>::parsed();
    /// let time = cli.custom.before_time.unwrap_or_default();
    ///
    /// MyWorld::cucumber()
    ///     .before(move |_, _, _, _| time::sleep(time).boxed_local())
    ///     .with_cli(cli)
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    /// ```
    /// <script
    ///     id="asciicast-0KvTxnfaMRjsvsIKsalS611Ta"
    ///     src="https://asciinema.org/a/0KvTxnfaMRjsvsIKsalS611Ta.js"
    ///     async data-autoplay="true" data-rows="14">
    /// </script>
    ///
    /// Also, specifying `--help` flag will describe `--before-time` now.
    ///
    /// [`Feature`]: gherkin::Feature
    #[must_use]
    pub fn with_cli<CustomCli>(
        self,
        cli: cli::Opts<P::Cli, R::Cli, Wr::Cli, CustomCli>,
    ) -> Cucumber<W, P, I, R, Wr, CustomCli>
    where
        CustomCli: clap::Args,
    {
        let Self { parser, runner, writer, filter_options, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: Some(cli),
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Initializes [`Default`] [`cli::Opts`].
    ///
    /// This method allows to omit parsing real [`cli::Opts`], as eagerly
    /// initializes [`Default`] ones instead.
    #[must_use]
    pub fn with_default_cli(mut self) -> Self
    where
        cli::Opts<P::Cli, R::Cli, Wr::Cli, Cli>: Default,
    {
        self.cli = Some(cli::Opts::default());
        self
    }

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// # Example
    ///
    /// Adjust [`Cucumber`] to run only [`Scenario`]s marked with `@cat` tag:
    /// ```rust
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .filter_run("tests/features/readme", |_, _, sc| {
    ///         sc.tags.iter().any(|t| t == "cat")
    ///     })
    ///     .await;
    /// # }
    /// ```
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   @cat
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   @dog
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    /// <script
    ///     id="asciicast-0KvTxnfaMRjsvsIKsalS611Ta"
    ///     src="https://asciinema.org/a/0KvTxnfaMRjsvsIKsalS611Ta.js"
    ///     async data-autoplay="true" data-rows="14">
    /// </script>
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    ///
    /// By default a CLI name or tag selector overrides `filter`, preserving the
    /// documented legacy CLI precedence. [`Self::with_filter_options`] opts
    /// into
    /// composition: selectors and this predicate must all accept the scenario.
    pub async fn filter_run<F>(self, input: I, filter: F) -> Wr
    where
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        let cli::Opts {
            re_filter,
            tags_filter,
            parser: parser_cli,
            runner: runner_cli,
            writer: writer_cli,
            ..
        } = self.cli.unwrap_or_else(cli::Opts::<_, _, _, _>::parsed);

        let filter_options = self.filter_options;
        let filter = move |feat: &gherkin::Feature,
                           rule: Option<&gherkin::Rule>,
                           scenario: &gherkin::Scenario| {
            if let Some(options) = &filter_options {
                return options.matches_with_cli(
                    feat,
                    rule,
                    scenario,
                    re_filter.as_ref(),
                    tags_filter.as_ref(),
                ) && filter(feat, rule, scenario);
            }
            re_filter.as_ref().map_or_else(
                || {
                    tags_filter.as_ref().map_or_else(
                        || filter(feat, rule, scenario),
                        |tags| {
                            // The order `Feature` -> `Rule` -> `Scenario`
                            // matters here.
                            tags.eval(
                                feat.tags
                                    .iter()
                                    .chain(rule.iter().flat_map(|r| &r.tags))
                                    .chain(scenario.tags.iter()),
                            )
                        },
                    )
                },
                |re| re.is_match(&scenario.name),
            )
        };

        let Self { parser, runner, mut writer, .. } = self;

        let features = parser.parse(input, parser_cli);

        let filtered = features.map(move |feature| {
            let mut feature = feature?;
            let feat_scenarios = mem::take(&mut feature.scenarios);
            feature.scenarios = feat_scenarios
                .into_iter()
                .filter(|s| filter(&feature, None, s))
                .collect();

            let mut rules = mem::take(&mut feature.rules);
            for r in &mut rules {
                let rule_scenarios = mem::take(&mut r.scenarios);
                r.scenarios = rule_scenarios
                    .into_iter()
                    .filter(|s| filter(&feature, Some(r), s))
                    .collect();
            }
            feature.rules = rules;

            Ok(feature)
        });

        let events_stream = runner.run(filtered, runner_cli);
        futures::pin_mut!(events_stream);
        while let Some(ev) = events_stream.next().await {
            writer.handle_event(ev, &writer_cli).await;
        }
        writer
    }
}

// Implemented manually to omit redundant `W: Clone` and `I: Clone` trait
// bounds, imposed by `#[derive(Clone)]`.
impl<W, P, I, R, Wr, Cli> Clone for Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Clone + Parser<I>,
    R: Clone + Runner<W>,
    Wr: Clone + Writer<W>,
    Cli: Clone + clap::Args,
    P::Cli: Clone,
    R::Cli: Clone,
    Wr::Cli: Clone,
{
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            runner: self.runner.clone(),
            writer: self.writer.clone(),
            cli: self.cli.clone(),
            filter_options: self.filter_options.clone(),
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

/// Shortcut for the [`Cucumber`] type returned by its [`Default`] impl.
pub(crate) type DefaultCucumber<W, I> = Cucumber<
    W,
    parser::Basic,
    I,
    runner::Basic<W>,
    writer::Summarize<writer::Normalize<W, writer::Basic>>,
>;

impl<W, I> Default for DefaultCucumber<W, I>
where
    W: World + Debug,
    I: AsRef<Path>,
{
    fn default() -> Self {
        Self::custom(
            parser::Basic::new(),
            runner::Basic::default(),
            writer::Basic::stdout().summarized(),
        )
    }
}

impl<W, I> DefaultCucumber<W, I>
where
    W: World + Debug,
    I: AsRef<Path>,
{
    /// Creates a default [`Cucumber`] executor.
    ///
    /// * [`Parser`] — [`parser::Basic`]
    ///
    /// * [`Runner`] — [`runner::Basic`]
    ///   * [`ScenarioType`] — [`Concurrent`] by default, [`Serial`] if
    ///     `@serial` [tag] is present on a [`Scenario`];
    ///   * Allowed to run up to 64 [`Concurrent`] [`Scenario`]s.
    ///
    /// * [`Writer`] — [`Normalize`] and [`Summarize`] [`writer::Basic`].
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Normalize`]: writer::Normalize
    /// [`Scenario`]: gherkin::Scenario
    /// [`Serial`]: ScenarioType::Serial
    /// [`Summarize`]: writer::Summarize
    ///
    /// [tag]: https://cucumber.io/docs/cucumber/api#tags
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<W, I, Wr, Cli, F, B, A>
    Cucumber<W, parser::Basic, I, runner::Basic<W, F, B, A>, Wr, Cli>
where
    W: World,
    runner::Basic<W, F, B, A>: Runner<W>,
    Wr: Writer<W>,
    Cli: clap::Args,
    I: AsRef<Path>,
{
    /// Selects the adopted Gherkin 43 parser policies and background execution.
    ///
    /// Valid featureless documents produce no scenarios. Empty scenarios omit
    /// background steps while preserving hooks. Step-definition matching
    /// remains
    /// its separately selected policy; this is not a Cucumber.js support
    /// bridge.
    ///
    /// # Errors
    /// Rejects a configured fallback language absent from the latest catalogue.
    pub fn with_gherkin43(
        mut self,
    ) -> Result<Self, parser::basic::UnsupportedLanguageError> {
        self.parser = self.parser.with_gherkin43()?;
        self.runner = self
            .runner
            .with_background_semantics(runner::BackgroundSemantics::Gherkin43);
        Ok(self)
    }
}

impl<W, I, R, Wr, Cli> Cucumber<W, parser::Basic, I, R, Wr, Cli>
where
    W: World,
    R: Runner<W>,
    Wr: Writer<W>,
    Cli: clap::Args,
    I: AsRef<Path>,
{
    /// Selects a complete ordered source plan instead of the run input/CLI
    /// glob.
    #[must_use]
    pub fn with_source_paths(mut self, paths: parser::SourcePaths) -> Self {
        self.parser = self.parser.with_source_paths(paths);
        self
    }

    /// Selects original source lines and composes all selection predicates.
    ///
    /// Enables empty [`crate::FilterOptions`] when no explicit typed options
    /// exist, so line selectors, name/tag selectors and the caller predicate
    /// all apply. Does not add files to the parser's independent discovery.
    #[must_use]
    pub fn with_line_selectors(
        mut self,
        selectors: crate::filter::LineSelectors,
    ) -> Self {
        self.parser = self.parser.with_line_selectors(selectors);
        self.filter_options.get_or_insert_with(crate::FilterOptions::new);
        self
    }

    /// Selects classic token whitespace and trimming without changing defaults.
    #[must_use]
    pub fn with_lexical_semantics(
        mut self,
        semantics: gherkin::LexicalSemantics,
    ) -> Self {
        self.parser = self.parser.with_lexical_semantics(semantics);
        self
    }

    /// Selects source syntax while preserving existing classic compatibility
    /// policies.
    ///
    /// Plain is the default. Markdown and Auto are explicit opt-ins; see
    /// [`parser::Basic::with_source_format`] for directory discovery behavior.
    #[must_use]
    pub fn with_source_format(mut self, format: parser::SourceFormat) -> Self {
        self.parser = self.parser.with_source_format(format);
        self
    }

    /// Captures exact source and raw AST in a shared per-run store.
    ///
    /// Pass the same store to a source-aware writer to report parsed bytes even
    /// if the source file later changes or disappears.
    #[must_use]
    pub fn with_source_store(mut self, sources: parser::SourceStore) -> Self {
        self.parser = self.parser.with_source_store(sources);
        self
    }

    /// Installs source-aware standard Messages reporting with a fresh source
    /// store.
    ///
    /// The Basic parser and Messages writer share exact parsed bytes and the
    /// raw AST. See [`writer::Messages`] for supported protocol semantics and
    /// remaining unsupported execution cases. Existing parser settings remain.
    /// Supporting runners also publish their final step registry at run start;
    /// custom runners without that capability keep definition metadata omitted.
    #[cfg(feature = "output-messages")]
    #[must_use]
    pub fn with_messages<O: std::io::Write>(
        mut self,
        output: O,
    ) -> Cucumber<
        W,
        parser::Basic,
        I,
        R,
        writer::Normalize<W, writer::Messages<O>>,
        Cli,
    > {
        let sources = parser::SourceStore::new();
        let support_code = crate::support_code::Store::new();
        self.runner = self.runner.with_support_code_store(support_code.clone());
        self.with_source_store(sources.clone()).with_writer(
            writer::Messages::raw_with_source_store(output, sources)
                .with_support_code_store(support_code)
                .normalized::<W>(),
        )
    }

    /// Selects table row sizing while preserving the legacy default.
    ///
    /// Gherkin43 rejects inconsistent step and Examples rows before execution.
    /// This selection leaves dialect, doc-string and interpolation policies
    /// intact.
    #[must_use]
    pub fn with_table_semantics(
        mut self,
        semantics: gherkin::TableSemantics,
    ) -> Self {
        self.parser = self.parser.with_table_semantics(semantics);
        self
    }

    /// Installs source-aware Messages reporting with an explicit protocol
    /// version.
    #[cfg(feature = "output-messages")]
    #[must_use]
    pub fn with_messages_protocol<O: std::io::Write>(
        mut self,
        output: O,
        protocol: writer::messages::ProtocolVersion,
    ) -> Cucumber<
        W,
        parser::Basic,
        I,
        R,
        writer::Normalize<W, writer::Messages<O>>,
        Cli,
    > {
        let sources = parser::SourceStore::new();
        let support_code = crate::support_code::Store::new();
        self.runner = self.runner.with_support_code_store(support_code.clone());
        self.with_source_store(sources.clone()).with_writer(
            writer::Messages::raw_with_source_store_and_protocol(
                output, sources, protocol,
            )
            .with_support_code_store(support_code)
            .normalized::<W>(),
        )
    }

    /// Selects Scenario Outline interpolation while preserving the legacy
    /// default.
    #[must_use]
    pub fn with_example_semantics(
        mut self,
        semantics: crate::feature::ExampleSemantics,
    ) -> Self {
        self.parser = self.parser.with_example_semantics(semantics);
        self
    }

    /// Selects the doc-string content delivered to step handlers.
    ///
    /// Legacy content is preserved by default. The explicit Gherkin 43 policy
    /// excludes media-type/header lines, dedents body lines, and unescapes the
    /// active delimiter before Scenario Outline values are substituted.
    #[must_use]
    pub fn with_docstring_semantics(
        mut self,
        semantics: gherkin::DocStringSemantics,
    ) -> Self {
        self.parser = self.parser.with_docstring_semantics(semantics);
        self
    }

    /// Selects a versioned dialect vocabulary for the Basic parser.
    ///
    /// This selects keyword vocabulary, not complete Gherkin runtime semantics.
    ///
    /// # Errors
    /// Returns an error if the current language is absent from the catalogue.
    pub fn with_dialect_catalog(
        mut self,
        catalog: parser::DialectCatalog,
    ) -> Result<Self, parser::basic::UnsupportedLanguageError> {
        self.parser = self.parser.with_dialect_catalog(catalog)?;
        Ok(self)
    }

    /// Sets the provided language of [`gherkin`] files.
    ///
    /// # Errors
    ///
    /// If the provided language isn't supported.
    pub fn language(
        mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<Self, parser::basic::UnsupportedLanguageError> {
        self.parser = self.parser.language(name)?;
        Ok(self)
    }
}

impl<W, I, P, Wr, F, B, A, Cli>
    Cucumber<W, P, I, runner::Basic<W, F, B, A>, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
    Cli: clap::Args,
    F: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> ScenarioType
        + 'static,
    B: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> LocalBoxFuture<'a, ()>
        + 'static,
    A: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a event::ScenarioFinished,
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>
        + 'static,
{
    /// If `max` is [`Some`] number of concurrently executed [`Scenario`]s will
    /// be limited.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn max_concurrent_scenarios(
        mut self,
        max: impl Into<Option<usize>>,
    ) -> Self {
        self.runner = self.runner.max_concurrent_scenarios(max);
        self
    }

    /// Makes failed [`Scenario`]s being retried the specified number of times.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retries(mut self, retries: impl Into<Option<usize>>) -> Self {
        self.runner = self.runner.retries(retries);
        self
    }

    /// Makes stop running tests on the first failure.
    ///
    /// __NOTE__: All the already started [`Scenario`]s at the moment of failure
    ///           will be finished.
    ///
    /// __NOTE__: Retried [`Scenario`]s are considered as failed, only in case
    ///           they exhaust all retry attempts and still do fail.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn fail_fast(mut self) -> Self {
        self.runner = self.runner.fail_fast();
        self
    }

    /// Makes failed [`Scenario`]s being retried after the specified
    /// [`Duration`] passes.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_after(mut self, after: impl Into<Option<Duration>>) -> Self {
        self.runner = self.runner.retry_after(after);
        self
    }

    /// Makes failed [`Scenario`]s being retried only if they're matching the
    /// specified `tag_expression`.
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_filter(
        mut self,
        tag_expression: impl Into<Option<TagOperation>>,
    ) -> Self {
        self.runner = self.runner.retry_filter(tag_expression);
        self
    }

    /// Function determining whether a [`Scenario`] is [`Concurrent`] or
    /// a [`Serial`] one.
    ///
    /// [`Concurrent`]: ScenarioType::Concurrent
    /// [`Serial`]: ScenarioType::Serial
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn which_scenario<Which>(
        self,
        func: Which,
    ) -> Cucumber<W, P, I, runner::Basic<W, Which, B, A>, Wr, Cli>
    where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let Self { parser, runner, writer, cli, filter_options, .. } = self;
        Cucumber {
            parser,
            runner: runner.which_scenario(func),
            writer,
            cli,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Function determining [`Scenario`]'s [`RetryOptions`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    #[must_use]
    pub fn retry_options<Retry>(mut self, func: Retry) -> Self
    where
        Retry: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
                &runner::basic::Cli,
            ) -> Option<RetryOptions>
            + 'static,
    {
        self.runner = self.runner.retry_options(func);
        self
    }

    /// Sets a hook, executed on each [`Scenario`] before running all its
    /// [`Step`]s, including [`Background`] ones.
    ///
    /// > **NOTE**: Only one [`before`] hook can be registered. If multiple
    /// >           calls are made, only the last one will be run.
    ///
    /// [`before`]: Self::before()
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn before<Before>(
        self,
        func: Before,
    ) -> Cucumber<W, P, I, runner::Basic<W, F, Before, A>, Wr, Cli>
    where
        Before: for<'a> Fn(
                &'a gherkin::Feature,
                Option<&'a gherkin::Rule>,
                &'a gherkin::Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let Self { parser, runner, writer, cli, filter_options, .. } = self;
        Cucumber {
            parser,
            runner: runner.before(func),
            writer,
            cli,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Sets a hook, executed on each [`Scenario`] after running all its
    /// [`Step`]s, even after [`Skipped`] of [`Failed`] [`Step`]s.
    ///
    /// > **NOTE**: Only one [`after`] hook can be registered. If multiple
    /// >           calls are made, only the last one will be run.
    ///
    /// Last `World` argument is supplied to the function, in case it was
    /// initialized before by running [`before`] hook or any [`Step`].
    ///
    /// [`after`]: Self::after()
    /// [`before`]: Self::before()
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn after<After>(
        self,
        func: After,
    ) -> Cucumber<W, P, I, runner::Basic<W, F, B, After>, Wr, Cli>
    where
        After: for<'a> Fn(
                &'a gherkin::Feature,
                Option<&'a gherkin::Rule>,
                &'a gherkin::Scenario,
                &'a event::ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let Self { parser, runner, writer, cli, filter_options, .. } = self;
        Cucumber {
            parser,
            runner: runner.after(func),
            writer,
            cli,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Installs reusable ordered scenario setup and teardown.
    ///
    /// Replaces both native stages. A later legacy [`Self::before()`] or
    /// [`Self::after()`] call replaces that stage. See [`runner::Hooks`] for
    /// ordering, filtering, failure behavior and reporting boundaries.
    #[must_use]
    pub fn with_hooks(
        self,
        hooks: runner::Hooks<W>,
    ) -> Cucumber<
        W,
        P,
        I,
        runner::Basic<W, F, runner::hooks::Before<W>, runner::hooks::After<W>>,
        Wr,
        Cli,
    > {
        let Self { parser, runner, writer, cli, filter_options, .. } = self;
        Cucumber {
            parser,
            runner: runner.with_hooks(hooks),
            writer,
            cli,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Collection`] of [`Step`]s.
    ///
    /// [`Collection`]: step::Collection
    /// [`Step`]: step::Step
    #[must_use]
    pub fn steps(mut self, steps: step::Collection<W>) -> Self {
        self.runner = self.runner.steps(steps);
        self
    }

    /// Enables runner-owned scopes in a shared typed-attachment store.
    ///
    /// Configure the Messages writer with the same store to report its records.
    /// Existing World constructors remain compatible; attachment-aware Worlds
    /// override World::new_with_context to retain their per-attempt context.
    #[must_use]
    pub fn with_attachments(
        mut self,
        store: crate::attachment::AttachmentStore,
    ) -> Self {
        self.runner = self.runner.with_attachments(store);
        self
    }

    /// Installs global setup/cleanup and counts their failures in run
    /// statistics.
    ///
    /// Select a custom writer before calling this method: replacing the writer
    /// afterward removes its statistics adapter. Protocol writers additionally
    /// consume the shared hook store to report individual global invocations.
    #[must_use]
    pub fn with_run_hooks(
        self,
        hooks: runner::RunHooks,
    ) -> Cucumber<W, P, I, runner::Basic<W, F, B, A>, writer::RunHooks<Wr>, Cli>
    {
        let store = hooks.store();
        let Self { parser, runner, writer, cli, filter_options, .. } = self;
        Cucumber {
            parser,
            runner: runner.with_run_hooks(hooks),
            writer: writer::RunHooks::new(writer, store),
            cli,
            filter_options,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Enables buffered global case ordering and retained retry reservations.
    #[must_use]
    pub fn scheduling_options(
        mut self,
        options: runner::SchedulingOptions,
    ) -> Self {
        self.runner = self.runner.with_scheduling_options(options);
        self
    }

    /// Selects run success rules for explicit pending and skipped outcomes.
    #[must_use]
    pub fn outcome_policy(
        mut self,
        policy: crate::execution::OutcomePolicy,
    ) -> Self {
        self.runner = self.runner.with_outcome_policy(policy);
        self
    }

    /// Selects execution or dry-run planning while preserving the legacy
    /// default.
    #[must_use]
    pub fn execution_mode(
        mut self,
        mode: crate::execution::ExecutionMode,
    ) -> Self {
        self.runner = self.runner.with_execution_mode(mode);
        self
    }

    /// Sets opt-in independent callback and step-body deadlines; the default
    /// is unlimited.
    #[must_use]
    pub fn with_timeouts(mut self, timeouts: runner::Timeouts) -> Self {
        self.runner = self.runner.with_timeouts(timeouts);
        self
    }

    /// Installs ordered per-step lifecycle callbacks without replacing
    /// scenario hooks.
    #[must_use]
    pub fn with_step_hooks(mut self, hooks: runner::StepHooks<W>) -> Self {
        self.runner = self.runner.with_step_hooks(hooks);
        self
    }

    /// Selects background execution for scenarios without their own steps.
    ///
    /// [`runner::BackgroundSemantics::Gherkin43`] matches official empty
    /// Pickles
    /// while preserving scenario hooks. The default retains legacy execution.
    #[must_use]
    pub fn with_background_semantics(
        mut self,
        semantics: runner::BackgroundSemantics,
    ) -> Self {
        self.runner = self.runner.with_background_semantics(semantics);
        self
    }

    /// Selects the step-definition matching policy.
    ///
    /// [`step::Matching::KeywordIndependent`] searches Given, When and Then
    /// registrations together, following Cucumber.js matching semantics.
    /// The default remains [`step::Matching::KeywordSpecific`]. Configure this
    /// after [`Self::steps()`], which replaces the policy with the collection.
    #[must_use]
    pub fn step_matching(mut self, matching: step::Matching) -> Self {
        self.runner = self.runner.step_matching(matching);
        self
    }

    /// Inserts [Given] [`Step`].
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[must_use]
    pub fn given(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.given(regex, step);
        self
    }

    /// Inserts [When] [`Step`].
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    #[must_use]
    pub fn when(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.when(regex, step);
        self
    }

    /// Inserts [Then] [`Step`].
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    #[must_use]
    pub fn then(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.then(regex, step);
        self
    }
}

impl<W, I, P, R, Wr, Cli> Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: writer::Stats<W> + writer::Normalized,
    Cli: clap::Args,
{
    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] [`Failed`].
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Feature`]: gherkin::Feature
    /// [`Step`]: gherkin::Step
    pub async fn run_and_exit(self, input: I) {
        self.filter_run_and_exit(input, |_, _, _| true).await;
    }

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] [`Failed`].
    ///
    /// # Example
    ///
    /// Adjust [`Cucumber`] to run only [`Scenario`]s marked with `@cat` tag:
    /// ```rust
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .filter_run_and_exit("tests/features/readme", |_, _, sc| {
    ///         sc.tags.iter().any(|t| t == "cat")
    ///     })
    ///     .await;
    /// # }
    /// ```
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   @cat
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   @dog
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    /// <script
    ///     id="asciicast-0KvTxnfaMRjsvsIKsalS611Ta"
    ///     src="https://asciinema.org/a/0KvTxnfaMRjsvsIKsalS611Ta.js"
    ///     async data-autoplay="true" data-rows="14">
    /// </script>
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    pub async fn filter_run_and_exit<Filter>(self, input: I, filter: Filter)
    where
        Filter: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        let writer = self.filter_run(input, filter).await;
        if writer.execution_has_failed() {
            let mut msg = Vec::with_capacity(4);

            let failed_steps = writer.failed_steps();
            if failed_steps > 0 {
                msg.push(format!(
                    "{failed_steps} step{} failed",
                    if failed_steps > 1 { "s" } else { "" },
                ));
            }

            let pending_failures = writer.pending_failures();
            if pending_failures > 0 {
                msg.push(format!(
                    "{pending_failures} scenario{} failed because pending is \
                        strict",
                    if pending_failures > 1 { "s" } else { "" },
                ));
            }

            let parsing_errors = writer.parsing_errors();
            if parsing_errors > 0 {
                msg.push(format!(
                    "{parsing_errors} parsing error{}",
                    if parsing_errors > 1 { "s" } else { "" },
                ));
            }

            let hook_errors = writer.hook_errors();
            if hook_errors > 0 {
                msg.push(format!(
                    "{hook_errors} hook error{}",
                    if hook_errors > 1 { "s" } else { "" },
                ));
            }

            panic!("{}", msg.join(", "));
        }
    }
}
