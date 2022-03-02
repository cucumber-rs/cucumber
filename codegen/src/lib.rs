// Copyright (c) 2020-2022  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
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
    clippy::equatable_if_let,
    clippy::exit,
    clippy::expect_used,
    clippy::fallible_impl_from,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast,
    clippy::fn_to_numeric_cast_any,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::imprecise_flops,
    clippy::index_refutable_slice,
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
    clippy::same_name_method,
    clippy::shadow_unrelated,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::trailing_empty_array,
    clippy::trivial_regex,
    clippy::undocumented_unsafe_blocks,
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
mod parameter;
mod world_init;

use proc_macro::TokenStream;

// TODO: Remove once tests run without complains about it.
#[cfg(test)]
mod actually_used_crates_in_tests {
    use async_trait as _;
    use cucumber as _;
    use derive_more as _;
    use futures as _;
    use tempfile as _;
    use tokio as _;
}

/// Helper macro for generating public shims for [`macro@given`], [`macro@when`]
/// and [`macro@then`] attributes.
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
        /// use cucumber::{given, when, World, WorldInit};
        ///
        /// #[derive(Debug, WorldInit)]
        /// struct MyWorld;
        ///
        /// #[async_trait(?Send)]
        /// impl World for MyWorld {
        ///     type Error = Infallible;
        ///
        ///     async fn new() -> Result<Self, Self::Error> {
        ///         Ok(Self)
        ///     }
        /// }
        ///
        /// #[given(regex = r"(\S+) is (\d+)")]
        /// #[when(expr = "{word} is {int}")]
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
        /// # Attribute arguments
        ///
        /// - `#[given(regex = "regex")]`
        ///
        ///   Uses [`Regex`] for matching the step. [`Regex`] is checked at
        ///   compile time to have valid syntax.
        ///
        /// - `#[given(expr = "cucumber-expression")]`
        ///
        ///   Uses [Cucumber Expression][1] for matching the step. It's checked
        ///   at compile time to have valid syntax.
        ///
        /// - `#[given("literal")]`
        ///
        ///   Matches the step with an **exact** literal only. Doesn't allow any
        ///   values capturing to use as function arguments.
        ///
        /// # Function arguments
        ///
        /// - First argument has to be mutable reference to the [`WorldInit`]
        ///   deriver (your [`World`] implementer).
        /// - Other argument's types have to implement [`FromStr`] or it has to
        ///   be a slice where the element type also implements [`FromStr`].
        /// - To use [`gherkin::Step`], name the argument as `step`,
        ///   **or** mark the argument with a `#[step]` attribute.
        ///
        /// ```rust
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
        /// #         Ok(Self)
        /// #     }
        /// # }
        /// #
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
        /// # Return value
        ///
        /// A function may also return a [`Result`], which [`Err`] is expected
        /// to implement [`Display`], so returning it will cause the step to
        /// fail.
        ///
        /// [`Display`]: std::fmt::Display
        /// [`FromStr`]: std::str::FromStr
        /// [`Regex`]: regex::Regex
        /// [`gherkin::Step`]: https://bit.ly/3j42hcd
        /// [`World`]: https://bit.ly/3j0aWw7
        /// [1]: cucumber_expressions
        #[proc_macro_attribute]
        pub fn $name(args: TokenStream, input: TokenStream) -> TokenStream {
            attribute::step(std::stringify!($name), args.into(), input.into())
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }
    };
}

/// Helper macro for generating public shim of [`macro@WorldInit`] deriving
/// macro consistently with the ones of [`macro@given`], [`macro@when`] and
/// [`macro@then`] attributes.
macro_rules! steps {
    ($($name:ident),*) => {
        /// Derive macro for tests auto-wiring.
        ///
        /// See [`macro@given`], [`macro@when`] and [`macro@then`] attributes
        /// for further details.
        #[proc_macro_derive(WorldInit)]
        pub fn derive_init(input: TokenStream) -> TokenStream {
            world_init::derive(input.into(), &[$(std::stringify!($name)),*])
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }

        $(step_attribute!($name);)*
    }
}

steps!(given, when, then);

/// In addition to [default parameters] of [Cucumber Expressions], you may
/// implement and use custom ones.
///
/// # Example
///
/// ```rust
/// # use std::{convert::Infallible};
/// #
/// # use async_trait::async_trait;
/// use cucumber::{given, when, Parameter, World, WorldInit};
/// use derive_more::{Deref, FromStr};
///
/// #[derive(Debug, WorldInit)]
/// struct MyWorld;
///
/// #[async_trait(?Send)]
/// impl World for MyWorld {
///     type Error = Infallible;
///
///     async fn new() -> Result<Self, Self::Error> {
///         Ok(Self)
///     }
/// }
///
/// #[given(regex = r"^(\S+) is (\d+)$")]
/// #[when(expr = "{word} is {u64}")]
/// fn test(w: &mut MyWorld, param: String, num: CustomU64) {
///     assert_eq!(param, "foo");
///     assert_eq!(*num, 0);
/// }
///
/// #[derive(Deref, FromStr, Parameter)]
/// #[param(regex = r"\d+", name = "u64")]
/// struct CustomU64(u64);
/// #
/// # #[tokio::main]
/// # async fn main() {
/// #     MyWorld::run("./tests/features/doctests.feature").await;
/// # }
/// ```
///
/// # Attribute arguments
///
/// - `#[param(regex = "regex")]`
///
///   [`Regex`] to match this parameter. Usually shouldn't contain any capturing
///   groups, but in case it requires to do so, only the first non-empty group
///   will be matched as the result.
///
/// - `#[param(name = "name")]` (optional)
///
///   Name of this parameter to reference it by. If not specified, then
///   lower-cased type name will be used by default.
///
/// [`Regex`]: regex::Regex
/// [Cucumber Expressions]: https://cucumber.github.io/cucumber-expressions
/// [default parameters]: cucumber_expressions::Expression#parameter-types
#[proc_macro_derive(Parameter, attributes(param))]
pub fn parameter(input: TokenStream) -> TokenStream {
    parameter::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
