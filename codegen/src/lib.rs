// Copyright (c) 2020  Brendan Molloy <brendan@bbqsrc.net>,
//                     Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                     Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![doc(
    html_logo_url = "https://avatars.githubusercontent.com/u/91469139?s=128",
    html_favicon_url = "https://avatars.githubusercontent.com/u/91469139?s=256"
)]
#![doc = include_str!("../README.md")]
#![deny(
    macro_use_extern_crate,
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    trivial_casts,
    trivial_numeric_casts
)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    clippy::as_conversions,
    clippy::branches_sharing_code,
    clippy::clone_on_ref_ptr,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::decimal_literal_representation,
    clippy::else_if_without_else,
    clippy::empty_line_after_outer_attr,
    clippy::exit,
    clippy::expect_used,
    clippy::fallible_impl_from,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::imprecise_flops,
    clippy::let_underscore_must_use,
    clippy::lossy_float_literal,
    clippy::map_err_ignore,
    clippy::mem_forget,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::mutex_integer,
    clippy::nonstandard_macro_braces,
    clippy::option_if_let_else,
    clippy::panic_in_result_fn,
    clippy::pedantic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::shadow_unrelated,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::trivial_regex,
    clippy::unimplemented,
    clippy::unnecessary_self_imports,
    clippy::unneeded_field_pattern,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::use_self,
    clippy::useless_let_if_seq,
    clippy::wildcard_enum_match_arm,
    future_incompatible,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    noop_method_call,
    semicolon_in_expressions_from_macros,
    unreachable_pub,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]

mod attribute;
mod derive;

use proc_macro::TokenStream;

/// Expands `given`, `when` and `then` proc-macro attributes.
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
        /// use cucumber::{given, World, WorldInit};
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
        /// # use cucumber::{gherkin::Step, given, World, WorldInit};
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

/// Expands `WorldInit` derive proc-macro and `given`, `when`, `then` proc-macro
/// attributes.
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
