//! Core Cucumber struct definition and basic constructors.

use std::marker::PhantomData;

use derive_more::with_trait::Debug;

use crate::{
    Parser, Runner, World, Writer, cli,
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
    pub(super) parser: P,

    /// [`Runner`] executing [`Scenario`]s and producing [`event`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) runner: R,

    /// [`Writer`] outputting [`event`]s to some output.
    pub(super) writer: Wr,

    /// CLI options this [`Cucumber`] has been run with.
    ///
    /// If empty, then will be parsed from a command line.
    pub(super) cli: Option<cli::Opts<P::Cli, R::Cli, Wr::Cli, Cli>>,

    /// Type of the [`World`] this [`Cucumber`] run on.
    #[debug(ignore)]
    pub(super) _world: PhantomData<W>,

    /// Type of the input consumed by [`Cucumber::parser`].
    #[debug(ignore)]
    pub(super) _parser_input: PhantomData<I>,
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
            _world: PhantomData,
            _parser_input: PhantomData,
        }
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
        let Self { runner, writer, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: None,
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
        let Self { parser, writer, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: None,
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
        let Self { parser, runner, .. } = self;
        Cucumber {
            parser,
            runner,
            writer,
            cli: None,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}