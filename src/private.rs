//! Helper type-level glue for [`cucumber_rust_codegen`] crate.

use std::{fmt::Debug, path::Path};

use async_trait::async_trait;
use sealed::sealed;

use crate::{
    parser, runner, runner::ScenarioType, step, writer, Cucumber, Step, World,
    WriterExt as _,
};

pub use futures::future::LocalBoxFuture;
pub use inventory::{self, collect, submit};
pub use regex::Regex;

/// [`World`] extension with auto-wiring capabilities.
pub trait WorldInit<G, W, T>: WorldInventory<G, W, T>
where
    G: StepConstructor<Self> + inventory::Collect,
    W: StepConstructor<Self> + inventory::Collect,
    T: StepConstructor<Self> + inventory::Collect,
{
    /// Returns runner for tests with auto-wired steps marked by [`given`],
    /// [`when`] and [`then`] attributes.
    ///
    /// [`given`]: crate::given
    /// [`then`]: crate::then
    /// [`when`]: crate::when
    #[must_use]
    fn collection() -> step::Collection<Self> {
        let mut collection = step::Collection::new();

        for given in Self::cucumber_given() {
            let (regex, fun) = given.inner();
            collection = collection.given(regex, fun);
        }

        for when in Self::cucumber_when() {
            let (regex, fun) = when.inner();
            collection = collection.when(regex, fun);
        }

        for then in Self::cucumber_then() {
            let (regex, fun) = then.inner();
            collection = collection.then(regex, fun);
        }

        collection
    }
}

impl<G, W, T, E> WorldInit<G, W, T> for E
where
    G: StepConstructor<Self> + inventory::Collect,
    W: StepConstructor<Self> + inventory::Collect,
    T: StepConstructor<Self> + inventory::Collect,
    E: WorldInventory<G, W, T>,
{
}

/// [`World`] extension with auto-wiring capabilities.
#[async_trait(?Send)]
#[sealed]
pub trait WorldRun<G, W, T>: WorldInit<G, W, T>
where
    Self: Debug,
    G: StepConstructor<Self> + inventory::Collect,
    W: StepConstructor<Self> + inventory::Collect,
    T: StepConstructor<Self> + inventory::Collect,
{
    async fn run<I: AsRef<Path>>(input: I) {
        let cucumber = Cucumber::custom(
            parser::Basic,
            runner::basic::Basic::new(
                |sc| {
                    sc.tags
                        .iter()
                        .any(|tag| tag == "serial")
                        .then(|| ScenarioType::Serial)
                        .unwrap_or(ScenarioType::Concurrent)
                },
                16,
                Self::collection(),
            ),
            writer::Basic::new().normalize().summarize(),
        );
        cucumber.run_and_exit(input).await;
    }
}

#[sealed]
impl<G, W, T, E> WorldRun<G, W, T> for E
where
    E: WorldInit<G, W, T> + Debug,
    G: StepConstructor<Self> + inventory::Collect,
    W: StepConstructor<Self> + inventory::Collect,
    T: StepConstructor<Self> + inventory::Collect,
{
}

/// [`World`] extension allowing to register steps in [`inventory`].
pub trait WorldInventory<G, W, T>: World
where
    G: StepConstructor<Self> + inventory::Collect,
    W: StepConstructor<Self> + inventory::Collect,
    T: StepConstructor<Self> + inventory::Collect,
{
    /// Returns [`Iterator`] over items with [`given`] attribute.
    ///
    /// [`given`]: crate::given
    /// [`Iterator`]: std::iter::Iterator
    #[must_use]
    fn cucumber_given() -> inventory::iter<G> {
        inventory::iter
    }

    /// Creates new [`Given`] [`Step`] value. Used by [`given`] attribute.
    ///
    /// [`given`]: crate::given
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    fn new_given(regex: Regex, fun: Step<Self>) -> G {
        G::new(regex, fun)
    }

    /// Returns [`Iterator`] over items with [`when`] attribute.
    ///
    /// [`when`]: crate::when
    /// [`Iterator`]: std::iter::Iterator
    #[must_use]
    fn cucumber_when() -> inventory::iter<W> {
        inventory::iter
    }

    /// Creates new [`When`] [`Step`] value. Used by [`when`] attribute.
    ///
    /// [`when`]: crate::when
    /// [When]: https://cucumber.io/docs/gherkin/reference/#when
    fn new_when(regex: Regex, fun: Step<Self>) -> W {
        W::new(regex, fun)
    }

    /// Returns [`Iterator`] over items with [`then`] attribute.
    ///
    /// [`then`]: crate::then
    /// [`Iterator`]: std::iter::Iterator
    #[must_use]
    fn cucumber_then() -> inventory::iter<T> {
        inventory::iter
    }

    /// Creates new [`Then`] [`Step`] value. Used by [`then`] attribute.
    ///
    /// [`then`]: crate::then
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    fn new_then(regex: Regex, fun: Step<Self>) -> T {
        T::new(regex, fun)
    }
}

/// Trait for creating [`Step`]s to be registered by [`given`], [`when`] and
/// [`then`] attributes.
///
/// [`given`]: crate::given
/// [`when`]: crate::when
/// [`then`]: crate::then
pub trait StepConstructor<W> {
    /// Creates new [`Step`] with corresponding [`Regex`].
    fn new(_: Regex, _: Step<W>) -> Self;

    /// Returns inner [`Step`] with corresponding [`Regex`].
    fn inner(&self) -> (Regex, Step<W>);
}
