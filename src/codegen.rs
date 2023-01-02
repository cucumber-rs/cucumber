// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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

/// Return-type polymorphism over `async`ness for a `#[world(init)]` attribute
/// of a [`#[derive(World)]`](macro@World) macro.
///
/// It allows to accept both sync and `async` functions as an attribute's
/// argument, by automatically wrapping sync functions in a [`future::Ready`].
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
///             IntoWorldResult as _, ToWorldFuture as _,
///         };
///
///         fn as_fn_ptr<T>(v: fn() -> T) -> fn() -> T {
///             v
///         }
///
///         //           `#[world(init)]`'s value
///         //          ⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄
///         (&as_fn_ptr(<Self as Default>::default))
///             .to_world_future() // maybe wraps into `future::Ready`
///             .await
///             .into_world_result()
///             .map_err(Into::into)
///     }
/// }
/// ```
pub trait ToWorldFuture {
    /// [`Future`] returned by this [`World`] constructor.
    ///
    /// Set to [`future::Ready`] in case construction is sync.
    type Future: Future;

    /// Resolves this [`Future`] for constructing a new[`World`] using
    /// [autoderef-based specialization][0].
    ///
    /// [0]: https://tinyurl.com/autoref-spec
    fn to_world_future(&self) -> Self::Future;
}

impl<R: IntoWorldResult> ToWorldFuture for fn() -> R {
    type Future = future::Ready<R>;

    fn to_world_future(&self) -> Self::Future {
        future::ready(self())
    }
}

impl<Fut: Future> ToWorldFuture for &fn() -> Fut
where
    Fut::Output: IntoWorldResult,
{
    type Future = Fut;

    fn to_world_future(&self) -> Self::Future {
        self()
    }
}

/// Return-type polymorphism over fallibility for a `#[world(init)]` attribute
/// of a [`#[derive(World)]`](macro@World) macro.
///
/// It allows to accept both fallible (returning [`Result`]) and infallible
/// functions as an attribute's argument, by automatically wrapping infallible
/// functions in a [`Result`]`<`[`World`]`, `[`Infallible`]`>`.
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
///             IntoWorldResult as _, ToWorldFuture as _,
///         };
///
///         fn as_fn_ptr<T>(v: fn() -> T) -> fn() -> T {
///             v
///         }
///
///         //           `#[world(init)]`'s value
///         //          ⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄
///         (&as_fn_ptr(<Self as Default>::default))
///             .to_world_future()
///             .await
///             .into_world_result() // maybe wraps into `Result<_, Infallible>`
///             .map_err(Into::into)
///     }
/// }
/// ```
pub trait IntoWorldResult: Sized {
    /// [`World`] type itself.
    type World: World;

    /// Error returned by this [`World`] constructor.
    ///
    /// Set to [`Infallible`] in case construction is infallible.
    type Error;

    /// Passes [`Result`]`<`[`World`]`, Self::Error>` as is, or wraps the plain
    /// [`World`] in a [`Result`]`<`[`World`]`, `[`Infallible`]`>`.
    ///
    /// # Errors
    ///
    /// In case the [`World`] construction errors.
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
