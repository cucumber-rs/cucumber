// Copyright (c) 2020  Brendan Molloy <brendan@bbqsrc.net>,
//                     Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                     Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![doc = include_str!("../README.md")]
#![deny(
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    trivial_casts,
    trivial_numeric_casts
)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    unused_import_braces,
    unused_labels,
    unused_qualifications,
    unused_results
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod attribute;
mod derive;

use proc_macro::TokenStream;

macro_rules! step_attribute {
    ($name:ident) => {
        /// Attribute to auto-wire the test to the [`World`] implementer.
        ///
        /// There are 3 step-specific attributes:
        /// - [`macro@given`]
        /// - [`macro@when`]
        /// - [`macro@then`]
        ///
        /// # Example
        ///
        /// ```
        /// # use std::{convert::Infallible};
        /// #
        /// # use async_trait::async_trait;
        /// use cucumber_rust::{given, World, WorldInit};
        ///
        /// #[derive(Debug, WorldInit)]
        /// struct MyWorld;
        ///
        /// #[async_trait(?Send)]
        /// impl World for MyWorld {
        ///     type Error = Infallible;
        ///
        ///     async fn new() -> Result<Self, Self::Error> {
        ///         Ok(Self {})
        ///     }
        /// }
        ///
        /// #[given(regex = r"(\S+) is (\d+)")]
        /// fn test(w: &mut MyWorld, param: String, num: i32) {
        ///     assert_eq!(param, "foo");
        ///     assert_eq!(num, 0);
        /// }
        ///
        /// #[tokio::main]
        /// async fn main() {
        ///     MyWorld::run("./tests/features/doctests.feature").await;
        /// }
        /// ```
        ///
        /// # Arguments
        ///
        /// - First argument has to be mutable reference to the [`WorldInit`]
        ///   deriver (your [`World`] implementer).
        /// - Other argument's types have to implement [`FromStr`] or it has to
        ///   be a slice where the element type also implements [`FromStr`].
        /// - To use [`gherkin::Step`], name the argument as `step`,
        ///   **or** mark the argument with a `#[step]` attribute.
        ///
        /// ```
        /// # use std::convert::Infallible;
        /// #
        /// # use async_trait::async_trait;
        /// # use cucumber_rust::{gherkin::Step, given, World, WorldInit};
        /// #
        /// # #[derive(Debug, WorldInit)]
        /// # struct MyWorld;
        /// #
        /// # #[async_trait(?Send)]
        /// # impl World for MyWorld {
        /// #     type Error = Infallible;
        /// #
        /// #     async fn new() -> Result<Self, Self::Error> {
        /// #         Ok(Self {})
        /// #     }
        /// # }
        ///
        /// #[given(regex = r"(\S+) is not (\S+)")]
        /// fn test_step(
        ///     w: &mut MyWorld,
        ///     #[step] s: &Step,
        ///     matches: &[String],
        /// ) {
        ///     assert_eq!(matches[0], "foo");
        ///     assert_eq!(matches[1], "bar");
        ///     assert_eq!(s.value, "foo is not bar");
        /// }
        /// #
        /// # #[tokio::main]
        /// # async fn main() {
        /// #     MyWorld::run("./tests/features/doctests.feature").await;
        /// # }
        /// ```
        ///
        /// [`FromStr`]: std::str::FromStr
        /// [`gherkin::Step`]: https://bit.ly/3j42hcd
        /// [`World`]: https://bit.ly/3j0aWw7
        #[proc_macro_attribute]
        pub fn $name(args: TokenStream, input: TokenStream) -> TokenStream {
            attribute::step(std::stringify!($name), args.into(), input.into())
                .unwrap_or_else(|e| e.to_compile_error())
                .into()
        }
    };
}

macro_rules! steps {
    ($($name:ident),*) => {
        /// Derive macro for tests auto-wiring.
        ///
        /// See [`macro@given`], [`macro@when`] and [`macro@then`] attributes
        /// for further details.
        #[proc_macro_derive(WorldInit)]
        pub fn derive_init(input: TokenStream) -> TokenStream {
            derive::world_init(input.into(), &[$(std::stringify!($name)),*])
                .unwrap_or_else(|e| e.to_compile_error())
                .into()
        }

        $(step_attribute!($name);)*
    }
}

steps!(given, when, then);
