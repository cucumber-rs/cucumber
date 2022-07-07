// Copyright (c) 2018-2022  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helper type-level glue for [`cucumber_codegen`] crate.

use std::{convert::Infallible, future::Future};

use futures::future;

use crate::{step, Step, World};

pub use anyhow;
pub use async_trait::async_trait;
pub use cucumber_expressions::{
    expand::parameters::Provider as ParametersProvider, Expression, Spanned,
};
pub use futures::future::LocalBoxFuture;
pub use inventory::{self, collect, submit};
pub use once_cell::sync::Lazy;
pub use regex::Regex;

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
/// attributes inside [`World::collection()`] method.
///
/// [`given`]: crate::given
/// [`when`]: crate::when
/// [`then`]: crate::then
pub trait StepConstructor<W> {
    /// Returns an inner [`Step`] with the corresponding [`Regex`].
    fn inner(&self) -> (step::Location, LazyRegex, Step<W>);
}

/// Custom parameter of a [Cucumber Expression].
///
/// Should be implemented only with via [`Parameter`] derive macro.
///
/// [`Parameter`]: macro@crate::Parameter
/// [Cucumber Expression]: https://cucumber.github.io/cucumber-expressions
pub trait Parameter {
    /// [`Regex`] matching this [`Parameter`].
    ///
    /// Shouldn't contain any capturing groups.
    ///
    /// Validated during [`Parameter`](macro@crate::Parameter) derive macro
    /// expansion.
    ///
    /// [`Regex`]: regex::Regex
    const REGEX: &'static str;

    /// Name of this [`Parameter`] to be referenced by in
    /// [Cucumber Expressions].
    ///
    /// [Cucumber Expressions]: https://cucumber.github.io/cucumber-expressions
    const NAME: &'static str;
}

/// Compares two strings in a `const` context.
///
/// As there is no `const impl Trait` and `l == r` calls [`Eq`], we have to use
/// a custom comparison function.
///
/// [`Eq`]: std::cmp::Eq
// TODO: Remove once `Eq` trait is allowed in a `const` context.
#[must_use]
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

/// Helper-trait for resolving `#[derive(`[`World`]`)]`'s `#[world(init)]`
/// attribute.
///
/// In combination with [`IntoWorldResult`] this allows us to accept async or
/// sync, fallible or infallible functions as a `#[world(init)]`'s value. This
/// is done by wrapping sync functions into [`future::Ready`] and infallible
/// into [`Result`]`<`[`World`]`, `[`Infallible`]`>`.
///
/// ```rust
/// # use async_trait::async_trait;
/// #
/// # #[derive(Default)]
/// # struct World;
/// #
/// #[async_trait(?Send)]
/// impl cucumber::World for World {
///     type Error = anyhow::Error;
///
///     async fn new() -> Result<Self, Self::Error> {
///         use cucumber::codegen::{
///             IntoWorldFuture as _, IntoWorldResult as _,
///         };
///
///         fn as_fn_ptr<T>(v: fn() -> T) -> fn() -> T {
///             v
///         }
///
///         //           `#[world(init)]`'s value
///         //          ⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄
///         (&as_fn_ptr(<Self as Default>::default))
///             .into_world_future() // Maybe wraps into `future::Ready`
///             .await
///             .into_world_result() // Maybe wraps into `Result<_, Infallible>`
///             .map_err(Into::into)
///     }
/// }
/// ```
///
/// [`World`]: macro@crate::World
pub trait IntoWorldFuture: Sized {
    /// [`Future`] returned by [`World`]'s constructor.
    ///
    /// Set to [`future::Ready`] in case construction is sync.
    type Future: Future;

    /// Resolves [`Future`] for constructing [`World`] using
    /// [autoderef-based specialization][1].
    ///
    /// [1]: https://bit.ly/3AuAfRp
    #[allow(clippy::wrong_self_convention)]
    fn into_world_future(&self) -> Self::Future;
}

impl<R: IntoWorldResult> IntoWorldFuture for fn() -> R {
    type Future = future::Ready<R>;

    fn into_world_future(&self) -> Self::Future {
        future::ready(self())
    }
}

impl<Fut: Future> IntoWorldFuture for &fn() -> Fut
where
    Fut::Output: IntoWorldResult,
{
    type Future = Fut;

    fn into_world_future(&self) -> Self::Future {
        self()
    }
}

/// Helper-trait for resolving `#[derive(`[`World`]`)]`'s `#[world(init)]`
/// attribute.
///
/// See [`IntoWorldFuture`] for more info.
///
/// [`World`]: macro@crate::World
pub trait IntoWorldResult: Sized {
    /// [`World`] type itself.
    type World: World;

    /// Error returned by [`World`]'s constructor.
    ///
    /// Set to [`Infallible`] in case construction never errors.
    type Error;

    /// Passes [`Result`]`<`[`World`]`, Err>` as is, or wraps plain [`World`]
    /// into [`Result`]`<`[`World`]`, `[`Infallible`]`>`.
    ///
    /// # Errors
    ///
    /// In case [`World`] construction errors.
    fn into_world_result(self) -> Result<Self::World, Self::Error>;
}

impl<W: World> IntoWorldResult for W {
    type World = Self;
    type Error = Infallible;

    fn into_world_result(self) -> Result<Self, Self::Error> {
        Ok(self)
    }
}

impl<W: World, E> IntoWorldResult for Result<W, E> {
    type World = W;
    type Error = E;

    fn into_world_result(self) -> Self {
        self
    }
}
