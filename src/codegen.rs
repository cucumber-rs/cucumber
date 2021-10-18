// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helper type-level glue for [`cucumber_codegen`] crate.

use std::{fmt::Debug, path::Path};

use async_trait::async_trait;

use crate::{cucumber::DefaultCucumber, step, Cucumber, Step, World};

pub use futures::future::LocalBoxFuture;
pub use inventory::{self, collect, submit};
pub use regex::Regex;

/// [`World`] extension with auto-wiring capabilities.
#[async_trait(?Send)]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub trait WorldInit<G, W, T>: WorldInventory<G, W, T>
where
    Self: Debug,
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
        let mut out = step::Collection::new();

        for given in Self::cucumber_given() {
            let (loc, regex, fun) = given.inner();
            out = out.given(loc, regex, fun);
        }

        for when in Self::cucumber_when() {
            let (loc, regex, fun) = when.inner();
            out = out.when(loc, regex, fun);
        }

        for then in Self::cucumber_then() {
            let (loc, regex, fun) = then.inner();
            out = out.then(loc, regex, fun);
        }

        out
    }

    /// Returns default [`Cucumber`] with all auto-wired [`Step`]s.
    #[must_use]
    fn cucumber<I: AsRef<Path>>() -> DefaultCucumber<Self, I> {
        Cucumber::new().steps(Self::collection())
    }

    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced by [`Parser`] are fed into [`Runner`] where the
    /// later produces events handled by [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] panicked.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Parser`]: crate::Parser
    /// [`Runner`]: crate::Runner
    /// [`Step`]: crate::Step
    /// [`Writer`]: crate::Writer
    async fn run<I: AsRef<Path>>(input: I) {
        Self::cucumber().run_and_exit(input).await;
    }

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced by [`Parser`] are fed into [`Runner`] where the
    /// later produces events handled by [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] panicked.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Parser`]: crate::Parser
    /// [`Runner`]: crate::Runner
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    /// [`Writer`]: crate::Writer
    async fn filter_run<I, F>(input: I, filter: F)
    where
        I: AsRef<Path>,
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        Self::cucumber().filter_run_and_exit(input, filter).await;
    }
}

impl<G, W, T, E> WorldInit<G, W, T> for E
where
    Self: Debug,
    G: StepConstructor<Self> + inventory::Collect,
    W: StepConstructor<Self> + inventory::Collect,
    T: StepConstructor<Self> + inventory::Collect,
    E: WorldInventory<G, W, T>,
{
}

/// [`World`] extension allowing to register steps in [`inventory`].
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub trait WorldInventory<G, W, T>: World
where
    G: StepConstructor<Self> + inventory::Collect,
    W: StepConstructor<Self> + inventory::Collect,
    T: StepConstructor<Self> + inventory::Collect,
{
    /// Returns an [`Iterator`] over items with [`given`] attribute.
    ///
    /// [`given`]: crate::given
    #[must_use]
    fn cucumber_given() -> inventory::iter<G> {
        inventory::iter
    }

    /// Creates a new [`Given`] [`Step`] value. Used by [`given`] attribute.
    ///
    /// [`given`]: crate::given
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    fn new_given(
        loc: Option<step::Location>,
        regex: Regex,
        fun: Step<Self>,
    ) -> G {
        G::new(loc, regex, fun)
    }

    /// Returns an [`Iterator`] over items with [`when`] attribute.
    ///
    /// [`when`]: crate::when
    #[must_use]
    fn cucumber_when() -> inventory::iter<W> {
        inventory::iter
    }

    /// Creates a new [`When`] [`Step`] value. Used by [`when`] attribute.
    ///
    /// [`when`]: crate::when
    /// [When]: https://cucumber.io/docs/gherkin/reference/#when
    fn new_when(
        loc: Option<step::Location>,
        regex: Regex,
        fun: Step<Self>,
    ) -> W {
        W::new(loc, regex, fun)
    }

    /// Returns an [`Iterator`] over items with [`then`] attribute.
    ///
    /// [`then`]: crate::then
    #[must_use]
    fn cucumber_then() -> inventory::iter<T> {
        inventory::iter
    }

    /// Creates a new [`Then`] [`Step`] value. Used by [`then`] attribute.
    ///
    /// [`then`]: crate::then
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    fn new_then(
        loc: Option<step::Location>,
        regex: Regex,
        fun: Step<Self>,
    ) -> T {
        T::new(loc, regex, fun)
    }
}

/// Trait for creating [`Step`]s to be registered by [`given`], [`when`] and
/// [`then`] attributes.
///
/// [`given`]: crate::given
/// [`when`]: crate::when
/// [`then`]: crate::then
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub trait StepConstructor<W> {
    /// Creates a new [`Step`] with the corresponding [`Regex`].
    #[must_use]
    fn new(_: Option<step::Location>, _: Regex, _: Step<W>) -> Self;

    /// Returns an inner [`Step`] with the corresponding [`Regex`].
    fn inner(&self) -> (Option<step::Location>, Regex, Step<W>);
}
