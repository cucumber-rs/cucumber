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

pub use cucumber_expressions::{
    expand::parameters::Provider, Expression, Spanned,
};
pub use futures::future::LocalBoxFuture;
pub use inventory::{self, collect, submit};
pub use once_cell::sync::Lazy;
pub use regex::Regex;

/// [`World`] extension with auto-wiring capabilities.
#[async_trait(?Send)]
pub trait WorldInit: Debug + WorldInventory {
    /// Returns runner for tests with auto-wired steps marked by [`given`],
    /// [`when`] and [`then`] attributes.
    ///
    /// [`given`]: crate::given
    /// [`then`]: crate::then
    /// [`when`]: crate::when
    #[must_use]
    fn collection() -> step::Collection<Self> {
        let mut out = step::Collection::new();

        for given in inventory::iter::<Self::Given> {
            let (loc, regex, fun) = given.inner();
            out = out.given(Some(loc), regex(), fun);
        }

        for when in inventory::iter::<Self::When> {
            let (loc, regex, fun) = when.inner();
            out = out.when(Some(loc), regex(), fun);
        }

        for then in inventory::iter::<Self::Then> {
            let (loc, regex, fun) = then.inner();
            out = out.then(Some(loc), regex(), fun);
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

impl<T> WorldInit for T where T: Debug + WorldInventory {}

/// [`World`] extension allowing to register steps in [`inventory`].
pub trait WorldInventory: World {
    /// Struct [`submit`]ted in a [`given`] macro.
    ///
    /// [`given`]: crate::given
    type Given: inventory::Collect + StepConstructor<Self>;

    /// Struct [`submit`]ted in a [`when`] macro.
    ///
    /// [`when`]: crate::when
    type When: inventory::Collect + StepConstructor<Self>;

    /// Struct [`submit`]ted in a [`then`] macro.
    ///
    /// [`then`]: crate::then
    type Then: inventory::Collect + StepConstructor<Self>;
}

/// Alias for a [`fn`] returning a [`Lazy`] [`Regex`].
pub type LazyRegex = fn() -> Regex;

/// Trait for registering a [`Step`] with [`given`], [`when`] and [`then`]
/// attributes inside [`WorldInit::collection()`] method.
///
/// [`given`]: crate::given
/// [`when`]: crate::when
/// [`then`]: crate::then
pub trait StepConstructor<W> {
    /// Returns an inner [`Step`] with the corresponding [`Regex`].
    fn inner(&self) -> (step::Location, LazyRegex, Step<W>);
}

/// Compares strings in `const` context.
///
/// As there is no `const impl Trait` and `l == r` calls [`Eq`], we have to
/// write custom comparison function.
///
/// [`Eq`]: std::cmp::Eq
// TODO: Remove once `Eq` trait is allowed in `const` context.
pub const fn str_eq(l: &str, r: &str) -> bool {
    if l.len() != r.len() {
        return false;
    }

    let (l, r) = (l.as_bytes(), r.as_bytes());
    let mut i = 0;
    while i < l.len() {
        if l[i] != r[i] {
            return false;
        }
        i += 1;
    }

    true
}
