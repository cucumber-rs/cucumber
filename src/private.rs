// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helper type-level glue for [`cucumber_rust_codegen`] crate.

use std::rc::Rc;

pub use inventory::{self, collect, submit};

use crate::{runner::TestFuture, Cucumber, Steps, World};

/// [`World`] extension with auto-wiring capabilities.
pub trait WorldInit<G1, G2, G3, G4, W1, W2, W3, W4, T1, T2, T3, T4>:
    WorldInventory<G1, G2, G3, G4, W1, W2, W3, W4, T1, T2, T3, T4>
where
    G1: Step<Self> + inventory::Collect,
    G2: StepRegex<Self> + inventory::Collect,
    G3: StepAsync<Self> + inventory::Collect,
    G4: StepRegexAsync<Self> + inventory::Collect,
    W1: Step<Self> + inventory::Collect,
    W2: StepRegex<Self> + inventory::Collect,
    W3: StepAsync<Self> + inventory::Collect,
    W4: StepRegexAsync<Self> + inventory::Collect,
    T1: Step<Self> + inventory::Collect,
    T2: StepRegex<Self> + inventory::Collect,
    T3: StepAsync<Self> + inventory::Collect,
    T4: StepRegexAsync<Self> + inventory::Collect,
{
    /// Returns runner for tests with auto-wired steps marked by [`given`], [`when`] and [`then`]
    /// attributes.
    ///
    /// [`given`]: crate::given
    /// [`then`]: crate::then
    /// [`when`]: crate::when
    #[must_use]
    fn init(features: &[&str]) -> Cucumber<Self> {
        Cucumber::new().features(features).steps({
            let mut builder: Steps<Self> = Steps::new();

            let (simple, regex, async_, async_regex) = Self::cucumber_given();
            for e in simple {
                let _ = builder.given(e.inner().0, e.inner().1);
            }
            for e in regex {
                let _ = builder.given_regex(e.inner().0, e.inner().1);
            }
            for e in async_ {
                let _ = builder.given_async(e.inner().0, e.inner().1);
            }
            for e in async_regex {
                let _ = builder.given_regex_async(e.inner().0, e.inner().1);
            }

            let (simple, regex, async_, async_regex) = Self::cucumber_when();
            for e in simple {
                let _ = builder.when(e.inner().0, e.inner().1);
            }
            for e in regex {
                let _ = builder.when_regex(e.inner().0, e.inner().1);
            }
            for e in async_ {
                let _ = builder.when_async(e.inner().0, e.inner().1);
            }
            for e in async_regex {
                let _ = builder.when_regex_async(e.inner().0, e.inner().1);
            }

            let (simple, regex, async_, async_regex) = Self::cucumber_then();
            for e in simple {
                let _ = builder.then(e.inner().0, e.inner().1);
            }
            for e in regex {
                let _ = builder.then_regex(e.inner().0, e.inner().1);
            }
            for e in async_ {
                let _ = builder.then_async(e.inner().0, e.inner().1);
            }
            for e in async_regex {
                let _ = builder.then_regex_async(e.inner().0, e.inner().1);
            }

            builder
        })
    }
}

impl<G1, G2, G3, G4, W1, W2, W3, W4, T1, T2, T3, T4, E>
    WorldInit<G1, G2, G3, G4, W1, W2, W3, W4, T1, T2, T3, T4> for E
where
    G1: Step<Self> + inventory::Collect,
    G2: StepRegex<Self> + inventory::Collect,
    G3: StepAsync<Self> + inventory::Collect,
    G4: StepRegexAsync<Self> + inventory::Collect,
    W1: Step<Self> + inventory::Collect,
    W2: StepRegex<Self> + inventory::Collect,
    W3: StepAsync<Self> + inventory::Collect,
    W4: StepRegexAsync<Self> + inventory::Collect,
    T1: Step<Self> + inventory::Collect,
    T2: StepRegex<Self> + inventory::Collect,
    T3: StepAsync<Self> + inventory::Collect,
    T4: StepRegexAsync<Self> + inventory::Collect,
    E: WorldInventory<G1, G2, G3, G4, W1, W2, W3, W4, T1, T2, T3, T4>,
{
}

/// [`World`] extension allowing to register steps in [`inventory`].
pub trait WorldInventory<G1, G2, G3, G4, W1, W2, W3, W4, T1, T2, T3, T4>: World
where
    G1: Step<Self> + inventory::Collect,
    G2: StepRegex<Self> + inventory::Collect,
    G3: StepAsync<Self> + inventory::Collect,
    G4: StepRegexAsync<Self> + inventory::Collect,
    W1: Step<Self> + inventory::Collect,
    W2: StepRegex<Self> + inventory::Collect,
    W3: StepAsync<Self> + inventory::Collect,
    W4: StepRegexAsync<Self> + inventory::Collect,
    T1: Step<Self> + inventory::Collect,
    T2: StepRegex<Self> + inventory::Collect,
    T3: StepAsync<Self> + inventory::Collect,
    T4: StepRegexAsync<Self> + inventory::Collect,
{
    #[must_use]
    fn cucumber_given() -> (
        inventory::iter<G1>,
        inventory::iter<G2>,
        inventory::iter<G3>,
        inventory::iter<G4>,
    ) {
        (
            inventory::iter,
            inventory::iter,
            inventory::iter,
            inventory::iter,
        )
    }

    fn new_given(name: &'static str, fun: CucumberFn<Self>) -> G1 {
        G1::new(name, fun)
    }

    fn new_given_regex(name: &'static str, fun: CucumberRegexFn<Self>) -> G2 {
        G2::new(name, fun)
    }

    fn new_given_async(name: &'static str, fun: CucumberAsyncFn<Self>) -> G3 {
        G3::new(name, fun)
    }

    fn new_given_regex_async(name: &'static str, fun: CucumberAsyncRegexFn<Self>) -> G4 {
        G4::new(name, fun)
    }

    #[must_use]
    fn cucumber_when() -> (
        inventory::iter<W1>,
        inventory::iter<W2>,
        inventory::iter<W3>,
        inventory::iter<W4>,
    ) {
        (
            inventory::iter,
            inventory::iter,
            inventory::iter,
            inventory::iter,
        )
    }

    fn new_when(name: &'static str, fun: CucumberFn<Self>) -> W1 {
        W1::new(name, fun)
    }

    fn new_when_regex(name: &'static str, fun: CucumberRegexFn<Self>) -> W2 {
        W2::new(name, fun)
    }

    fn new_when_async(name: &'static str, fun: CucumberAsyncFn<Self>) -> W3 {
        W3::new(name, fun)
    }

    fn new_when_regex_async(name: &'static str, fun: CucumberAsyncRegexFn<Self>) -> W4 {
        W4::new(name, fun)
    }

    #[must_use]
    fn cucumber_then() -> (
        inventory::iter<T1>,
        inventory::iter<T2>,
        inventory::iter<T3>,
        inventory::iter<T4>,
    ) {
        (
            inventory::iter,
            inventory::iter,
            inventory::iter,
            inventory::iter,
        )
    }

    fn new_then(name: &'static str, fun: CucumberFn<Self>) -> T1 {
        T1::new(name, fun)
    }

    fn new_then_regex(name: &'static str, fun: CucumberRegexFn<Self>) -> T2 {
        T2::new(name, fun)
    }

    fn new_then_async(name: &'static str, fun: CucumberAsyncFn<Self>) -> T3 {
        T3::new(name, fun)
    }

    fn new_then_regex_async(name: &'static str, fun: CucumberAsyncRegexFn<Self>) -> T4 {
        T4::new(name, fun)
    }
}

pub trait Step<W> {
    fn new(_: &'static str, _: CucumberFn<W>) -> Self;
    fn inner(&self) -> (&'static str, CucumberFn<W>);
}

pub trait StepRegex<W> {
    fn new(_: &'static str, _: CucumberRegexFn<W>) -> Self;
    fn inner(&self) -> (&'static str, CucumberRegexFn<W>);
}

pub trait StepAsync<W> {
    fn new(_: &'static str, _: CucumberAsyncFn<W>) -> Self;
    fn inner(&self) -> (&'static str, CucumberAsyncFn<W>);
}

pub trait StepRegexAsync<W> {
    fn new(_: &'static str, _: CucumberAsyncRegexFn<W>) -> Self;
    fn inner(&self) -> (&'static str, CucumberAsyncRegexFn<W>);
}

pub type CucumberFn<W> = fn(W, Rc<gherkin::Step>) -> W;

pub type CucumberRegexFn<W> = fn(W, Vec<String>, Rc<gherkin::Step>) -> W;

pub type CucumberAsyncFn<W> = fn(W, Rc<gherkin::Step>) -> TestFuture<W>;

pub type CucumberAsyncRegexFn<W> = fn(W, Vec<String>, Rc<gherkin::Step>) -> TestFuture<W>;
