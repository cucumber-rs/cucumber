//! CLI configuration methods for Cucumber executor.

use std::marker::PhantomData;

use crate::{
    Parser, Runner, World, Writer, cli, writer,
};

use super::core::Cucumber;

impl<W, P, I, R, Wr, Cli> Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W> + writer::Normalized,
    Cli: clap::Args,
{
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
        let Self { parser, runner, writer, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: Some(cli),
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
}