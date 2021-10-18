// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
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

use std::{
    borrow::Cow,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem,
    path::Path,
};

use clap::Clap as _;
use futures::{future::LocalBoxFuture, StreamExt as _};
use regex::Regex;

use crate::{
    cli, event, parser, runner, step, writer, ArbitraryWriter, FailureWriter,
    Parser, Runner, ScenarioType, Step, World, Writer, WriterExt as _,
};

/// Top-level [Cucumber] executor.
///
/// Most of the time you don't need to work with it directly, just use
/// [`WorldInit::run()`] or [`WorldInit::cucumber()`] on your [`World`] deriver
/// to get [Cucumber] up and running.
///
/// Otherwise use [`Cucumber::new()`] to get the default [Cucumber] executor,
/// provide [`Step`]s with [`WorldInit::collection()`] or by hand with
/// [`Cucumber::given()`], [`Cucumber::when()`] and [`Cucumber::then()`].
///
/// In case you want custom [`Parser`], [`Runner`] or [`Writer`] or
/// some other finer control,  use [`Cucumber::custom()`] with
/// [`Cucumber::with_parser()`], [`Cucumber::with_runner()`] and
/// [`Cucumber::with_writer()`] to construct your dream [Cucumber] executor!
///
/// [Cucumber]: https://cucumber.io
/// [`WorldInit::collection()`]: crate::WorldInit::collection()
/// [`WorldInit::cucumber()`]: crate::WorldInit::cucumber()
/// [`WorldInit::run()`]: crate::WorldInit::run()
pub struct Cucumber<W, P, I, R, Wr> {
    parser: P,
    runner: R,
    writer: Wr,
    _world: PhantomData<W>,
    _parser_input: PhantomData<I>,
}

impl<W> Cucumber<W, (), (), (), ()> {
    /// Creates an empty [`Cucumber`] executor.
    ///
    /// Use [`Cucumber::with_parser()`], [`Cucumber::with_runner()`] and
    /// [`Cucumber::with_writer()`] to be able to [`Cucumber::run()`] it.
    #[must_use]
    pub fn custom() -> Self {
        Self {
            parser: (),
            runner: (),
            writer: (),
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

impl<W, P, I, R, Wr> Cucumber<W, P, I, R, Wr> {
    /// Replaces [`Parser`].
    #[must_use]
    pub fn with_parser<NewP, NewI>(
        self,
        parser: NewP,
    ) -> Cucumber<W, NewP, NewI, R, Wr>
    where
        NewP: Parser<NewI>,
    {
        let Self { runner, writer, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Runner`].
    #[must_use]
    pub fn with_runner<NewR>(self, runner: NewR) -> Cucumber<W, P, I, NewR, Wr>
    where
        NewR: Runner<W>,
    {
        let Self { parser, writer, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Replaces [`Writer`].
    #[must_use]
    pub fn with_writer<NewWr>(
        self,
        writer: NewWr,
    ) -> Cucumber<W, P, I, R, NewWr>
    where
        NewWr: Writer<W>,
    {
        let Self { parser, runner, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

impl<W, P, I, R, Wr> Cucumber<W, P, I, R, Wr>
where
    W: World,
    Wr: Writer<W>,
{
    /// Consider [`Skipped`] steps as [`Failed`] if their [`Scenario`] isn't
    /// marked with `@allow_skipped` tag.
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
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use cucumber::WorldInit;
    /// # use futures::FutureExt as _;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
    /// ```
    /// <script
    ///     id="asciicast-IHLxMEgku9BtBVkR4k2DtOjMd"
    ///     src="https://asciinema.org/a/IHLxMEgku9BtBVkR4k2DtOjMd.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// To intentionally suppress some [`Skipped`] steps failing, use the
    /// `@allow_skipped` tag:
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   @allow_skipped
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    ///
    /// [`Failed`]: crate::event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: crate::event::Step::Skipped
    #[must_use]
    pub fn fail_on_skipped(
        self,
    ) -> Cucumber<W, P, I, R, writer::FailOnSkipped<Wr>> {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.fail_on_skipped(),
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Consider [`Skipped`] steps as [`Failed`] if the given `filter` predicate
    /// returns `true`.
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
    /// marked with `@dog` tag:
    /// ```rust,should_panic
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped_with(|_, _, s| !s.tags.iter().any(|t| t == "dog"))
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
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
    /// [`Failed`]: crate::event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: crate::event::Step::Skipped
    #[must_use]
    pub fn fail_on_skipped_with<Filter>(
        self,
        filter: Filter,
    ) -> Cucumber<W, P, I, R, writer::FailOnSkipped<Wr, Filter>>
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
    /// Adjust [`Cucumber`] to re-output all [`Skipped`] steps at the end:
    /// ```rust
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use futures::FutureExt as _;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .repeat_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
    /// ```
    /// <script
    ///     id="asciicast-ox14HynkBIw8atpfhyfvKrsO3"
    ///     src="https://asciinema.org/a/ox14HynkBIw8atpfhyfvKrsO3.js"
    ///     async data-autoplay="true" data-rows="19">
    /// </script>
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: crate::event::Step::Skipped
    #[must_use]
    pub fn repeat_skipped(self) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr>> {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_skipped(),
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
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use futures::FutureExt as _;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
    /// ```
    /// <script
    ///     id="asciicast-UcipuopO6IFEsIDty6vaJlCH9"
    ///     src="https://asciinema.org/a/UcipuopO6IFEsIDty6vaJlCH9.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all [`Failed`] steps at the end:
    /// ```rust,should_panic
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use futures::FutureExt as _;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .repeat_failed()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
    /// ```
    /// <script
    ///     id="asciicast-ofOljvyEMb41OTLhE081QKv68"
    ///     src="https://asciinema.org/a/ofOljvyEMb41OTLhE081QKv68.js"
    ///     async data-autoplay="true" data-rows="24">
    /// </script>
    ///
    /// > ⚠️ __WARNING__: [`Cucumber::repeat_failed()`] should be called before
    ///                   [`Cucumber::fail_on_skipped()`], as events pass from
    ///                   outer [`Writer`]s to inner ones. So we need to
    ///                   transform [`Skipped`] to [`Failed`] first, and only
    ///                   then [`Repeat`] them.
    ///
    /// [`Failed`]: crate::event::Step::Failed
    /// [`Repeat`]: writer::Repeat
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: crate::event::Step::Skipped
    #[must_use]
    pub fn repeat_failed(self) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr>> {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_failed(),
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Re-output steps by the given `filter` predicate.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::fail_on_skipped()`]:
    /// ```rust,should_panic
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use futures::FutureExt as _;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
    /// ```
    /// <script
    ///     id="asciicast-UcipuopO6IFEsIDty6vaJlCH9"
    ///     src="https://asciinema.org/a/UcipuopO6IFEsIDty6vaJlCH9.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all [`Failed`] steps ta the end by
    /// providing a custom `filter` predicate:
    /// ```rust,should_panic
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use futures::FutureExt as _;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .repeat_if(|ev| {
    ///         use cucumber::event::{Cucumber, Feature, Rule, Scenario, Step};
    ///
    ///         matches!(
    ///             ev,
    ///             Ok(Cucumber::Feature(
    ///                 _,
    ///                 Feature::Rule(
    ///                     _,
    ///                     Rule::Scenario(
    ///                         _,
    ///                         Scenario::Step(_, Step::Failed(..))
    ///                             | Scenario::Background(_, Step::Failed(..))
    ///                     )
    ///                 ) | Feature::Scenario(
    ///                     _,
    ///                     Scenario::Step(_, Step::Failed(..))
    ///                         | Scenario::Background(_, Step::Failed(..))
    ///                 )
    ///             )) | Err(_)
    ///         )
    ///     })
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
    /// ```
    /// <script
    ///     id="asciicast-ofOljvyEMb41OTLhE081QKv68"
    ///     src="https://asciinema.org/a/ofOljvyEMb41OTLhE081QKv68.js"
    ///     async data-autoplay="true" data-rows="24">
    /// </script>
    ///
    /// > ⚠️ __WARNING__: [`Cucumber::repeat_if()`] should be called before
    ///                   [`Cucumber::fail_on_skipped()`], as events pass from
    ///                   outer [`Writer`]s to inner ones. So we need to
    ///                   transform [`Skipped`] to [`Failed`] first, and only
    ///                   then [`Repeat`] them.
    ///
    /// [`Failed`]: crate::event::Step::Failed
    /// [`Repeat`]: writer::Repeat
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: crate::event::Step::Skipped
    #[must_use]
    pub fn repeat_if<F>(
        self,
        filter: F,
    ) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr, F>>
    where
        F: Fn(&parser::Result<event::Cucumber<W>>) -> bool,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_if(filter),
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}

impl<W, P, I, R, Wr> Cucumber<W, P, I, R, Wr>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W>,
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

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// # Example
    ///
    /// Adjust [`Cucumber`] to run only [`Scenario`]s marked with `@cat` tag:
    /// ```rust
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .filter_run("tests/features/readme", |_, _, sc| {
    ///         sc.tags.iter().any(|t| t == "cat")
    ///     })
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
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
    #[allow(clippy::non_ascii_literal)]
    pub async fn filter_run<F>(self, input: I, filter: F) -> Wr
    where
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        let opts = cli::Opts::parse();
        if opts.nocapture {
            eprintln!(
                "WARNING ⚠️: This option does nothing at the moment and is \
                             deprecated for removal in the next major release. \
                             Any output of step functions is not captured by \
                             default.",
            );
        }
        if opts.debug {
            eprintln!(
                "WARNING ⚠️: This option does nothing at the moment and is \
                             deprecated for removal in the next major release.",
            );
        }
        let filter = move |f: &gherkin::Feature,
                           r: Option<&gherkin::Rule>,
                           s: &gherkin::Scenario| {
            opts.filter
                .as_ref()
                .map_or_else(|| filter(f, r, s), |f| f.is_match(&s.name))
        };

        let Cucumber {
            parser,
            runner,
            mut writer,
            ..
        } = self;

        let features = parser.parse(input);

        let filtered = features.map(move |feature| {
            let mut feature = feature?;
            let scenarios = mem::take(&mut feature.scenarios);
            feature.scenarios = scenarios
                .into_iter()
                .filter(|s| filter(&feature, None, s))
                .collect();

            let mut rules = mem::take(&mut feature.rules);
            for r in &mut rules {
                let scenarios = mem::take(&mut r.scenarios);
                r.scenarios = scenarios
                    .into_iter()
                    .filter(|s| filter(&feature, Some(r), s))
                    .collect();
            }
            feature.rules = rules;

            Ok(feature)
        });

        let events_stream = runner.run(filtered);
        futures::pin_mut!(events_stream);
        while let Some(ev) = events_stream.next().await {
            writer.handle_event(ev).await;
        }
        writer
    }
}

impl<W, P, I, R, Wr> Debug for Cucumber<W, P, I, R, Wr>
where
    P: Debug,
    R: Debug,
    Wr: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cucumber")
            .field("parser", &self.parser)
            .field("runner", &self.runner)
            .field("writer", &self.writer)
            .finish()
    }
}

/// Shortcut for the [`Cucumber`] type returned by its [`Default`] impl.
pub(crate) type DefaultCucumber<W, I> = Cucumber<
    W,
    parser::Basic,
    I,
    runner::Basic<W>,
    writer::Summarized<writer::Normalized<W, writer::Basic>>,
>;

impl<W, I> Default for DefaultCucumber<W, I>
where
    W: World + Debug,
    I: AsRef<Path>,
{
    fn default() -> Self {
        Cucumber::custom()
            .with_parser(parser::Basic::new())
            .with_runner(runner::Basic::default())
            .with_writer(writer::Basic::new().normalized().summarized())
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
    /// * [`Writer`] — [`Normalized`] and [`Summarized`] [`writer::Basic`].
    ///
    /// [`Concurrent`]: runner::basic::ScenarioType::Concurrent
    /// [`Normalized`]: writer::Normalized
    /// [`Parser`]: parser::Parser
    /// [`Scenario`]: gherkin::Scenario
    /// [`Serial`]: runner::basic::ScenarioType::Serial
    /// [`ScenarioType`]: runner::basic::ScenarioType
    /// [`Summarized`]: writer::Summarized
    ///
    /// [tag]: https://cucumber.io/docs/cucumber/api/#tags
    #[must_use]
    pub fn new() -> Self {
        Cucumber::default()
    }
}

impl<W, I, R, Wr> Cucumber<W, parser::Basic, I, R, Wr> {
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

impl<W, I, P, Wr, F, B, A> Cucumber<W, P, I, runner::Basic<W, F, B, A>, Wr> {
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
    ) -> Cucumber<W, P, I, runner::Basic<W, Which, B, A>, Wr>
    where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let Self {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.which_scenario(func),
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Sets a hook, executed on each [`Scenario`] before running all its
    /// [`Step`]s, including [`Background`] ones.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn before<Before>(
        self,
        func: Before,
    ) -> Cucumber<W, P, I, runner::Basic<W, F, Before, A>, Wr>
    where
        Before: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            &'a mut W,
        ) -> LocalBoxFuture<'a, ()>,
    {
        let Self {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.before(func),
            writer,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Sets a hook, executed on each [`Scenario`] after running all its
    /// [`Step`]s, even after [`Skipped`] of [`Failed`] [`Step`]s.
    ///
    /// Last `World` argument is supplied to the function, in case it was
    /// initialized before by running [`before`] hook or any non-failed
    /// [`Step`]. In case the last [`Scenario`]'s [`Step`] failed, we want to
    /// return event with an exact `World` state. Also, we don't want to impose
    /// additional [`Clone`] bounds on `World`, so the only option left is to
    /// pass [`None`] to the function.
    ///
    ///
    /// [`before`]: Self::before()
    /// [`Failed`]: event::Step::Failed
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn after<After>(
        self,
        func: After,
    ) -> Cucumber<W, P, I, runner::Basic<W, F, B, After>, Wr>
    where
        After: for<'a> Fn(
            &'a gherkin::Feature,
            Option<&'a gherkin::Rule>,
            &'a gherkin::Scenario,
            Option<&'a mut W>,
        ) -> LocalBoxFuture<'a, ()>,
    {
        let Self {
            parser,
            runner,
            writer,
            ..
        } = self;
        Cucumber {
            parser,
            runner: runner.after(func),
            writer,
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

    /// Inserts [Given] [`Step`].
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    #[must_use]
    pub fn given(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.given(regex, step);
        self
    }

    /// Inserts [When] [`Step`].
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#When
    #[must_use]
    pub fn when(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.when(regex, step);
        self
    }

    /// Inserts [Then] [`Step`].
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    #[must_use]
    pub fn then(mut self, regex: Regex, step: Step<W>) -> Self {
        self.runner = self.runner.then(regex, step);
        self
    }
}

impl<W, I, P, R, Wr> Cucumber<W, P, I, R, Wr>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: for<'val> ArbitraryWriter<'val, W, String> + FailureWriter<W>,
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
    /// [`Failed`]: crate::event::Step::Failed
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
    /// # use std::convert::Infallible;
    /// #
    /// # use async_trait::async_trait;
    /// # use cucumber::WorldInit;
    /// #
    /// # #[derive(Debug, WorldInit)]
    /// # struct MyWorld;
    /// #
    /// # #[async_trait(?Send)]
    /// # impl cucumber::World for MyWorld {
    /// #     type Error = Infallible;
    /// #
    /// #     async fn new() -> Result<Self, Self::Error> {
    /// #         Ok(Self)
    /// #     }
    /// # }
    /// #
    /// # let fut = async {
    /// MyWorld::cucumber()
    ///     .filter_run_and_exit("tests/features/readme", |_, _, sc| {
    ///         sc.tags.iter().any(|t| t == "cat")
    ///     })
    ///     .await;
    /// # };
    /// #
    /// # futures::executor::block_on(fut);
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
    /// [`Failed`]: crate::event::Step::Failed
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: crate::Step
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
            let mut msg = Vec::with_capacity(3);

            let failed_steps = writer.failed_steps();
            if failed_steps > 0 {
                msg.push(format!(
                    "{} step{} failed",
                    failed_steps,
                    (failed_steps > 1).then(|| "s").unwrap_or_default(),
                ));
            }

            let parsing_errors = writer.parsing_errors();
            if parsing_errors > 0 {
                msg.push(format!(
                    "{} parsing error{}",
                    parsing_errors,
                    (parsing_errors > 1).then(|| "s").unwrap_or_default(),
                ));
            }

            let hook_errors = writer.hook_errors();
            if hook_errors > 0 {
                msg.push(format!(
                    "{} hook error{}",
                    hook_errors,
                    (hook_errors > 1).then(|| "s").unwrap_or_default(),
                ));
            }

            panic!("{}", msg.join(", "));
        }
    }
}
