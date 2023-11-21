// Copyright (c) 2020-2023  Brendan Molloy <brendan@bbqsrc.net>,
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
    rustdoc::all,
    trivial_casts,
    trivial_numeric_casts
)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    clippy::absolute_paths,
    clippy::as_conversions,
    clippy::as_ptr_cast_mut,
    clippy::assertions_on_result_states,
    clippy::branches_sharing_code,
    clippy::clear_with_drain,
    clippy::clone_on_ref_ptr,
    clippy::collection_is_never_read,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::decimal_literal_representation,
    clippy::default_union_representation,
    clippy::derive_partial_eq_without_eq,
    clippy::else_if_without_else,
    clippy::empty_drop,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::equatable_if_let,
    clippy::exit,
    clippy::expect_used,
    clippy::fallible_impl_from,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast,
    clippy::fn_to_numeric_cast_any,
    clippy::format_push_string,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::implied_bounds_in_impls,
    clippy::imprecise_flops,
    clippy::index_refutable_slice,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::iter_with_drain,
    clippy::large_include_file,
    clippy::large_stack_frames,
    clippy::let_underscore_untyped,
    clippy::lossy_float_literal,
    clippy::manual_clamp,
    clippy::map_err_ignore,
    clippy::mem_forget,
    clippy::missing_assert_message,
    clippy::missing_asserts_for_indexing,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::multiple_unsafe_ops_per_block,
    clippy::mutex_atomic,
    clippy::mutex_integer,
    clippy::needless_collect,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_raw_strings,
    clippy::nonstandard_macro_braces,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic_in_result_fn,
    clippy::partial_pub_fields,
    clippy::pedantic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_without_shorthand,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::readonly_write_lock,
    clippy::redundant_clone,
    clippy::redundant_type_annotations,
    clippy::ref_patterns,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::semicolon_inside_block,
    clippy::shadow_unrelated,
    clippy::significant_drop_in_scrutinee,
    clippy::significant_drop_tightening,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_lit_chars_any,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::suspicious_xor_used_as_pow,
    clippy::tests_outside_test_module,
    clippy::todo,
    clippy::trailing_empty_array,
    clippy::transmute_undefined_repr,
    clippy::trivial_regex,
    clippy::try_err,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    clippy::unnecessary_self_imports,
    clippy::unnecessary_struct_initialization,
    clippy::unneeded_field_pattern,
    clippy::unused_peekable,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::use_self,
    clippy::useless_let_if_seq,
    clippy::verbose_file_reads,
    clippy::wildcard_enum_match_arm,
    future_incompatible,
    let_underscore_drop,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    semicolon_in_expressions_from_macros,
    unreachable_pub,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    unused_tuple_struct_fields,
    variant_size_differences
)]

mod attribute;
mod parameter;
mod world;

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
        /// use cucumber::{given, when, World};
        ///
        /// #[derive(Debug, Default, World)]
        /// struct MyWorld;
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
        /// - First argument has to be mutable reference to the [`World`]
        ///   deriver.
        /// - Other argument's types have to implement [`FromStr`] or it has to
        ///   be a slice where the element type also implements [`FromStr`].
        /// - To use [`gherkin::Step`], name the argument as `step`,
        ///   **or** mark the argument with a `#[step]` attribute.
        ///
        /// ```rust
        /// # use std::convert::Infallible;
        /// #
        /// # use cucumber::{gherkin::Step, given, World};
        /// #
        /// # #[derive(Debug, Default, World)]
        /// # struct MyWorld;
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

/// Helper macro for generating public shim of [`macro@given`], [`macro@when`]
/// and [`macro@then`] attributes.
macro_rules! steps {
    ($($name:ident),*) => {
        $(step_attribute!($name);)*
    }
}

steps!(given, when, then);

/// Derive macro for implementing a [`World`] trait.
///
/// # Example
///
/// ```rust
/// #[derive(cucumber::World)]
/// #[world(init = Self::new)] // optional, uses `Default::default()` if omitted
/// struct World(usize);
///
/// impl World {
///     fn new() -> Self {
///         Self(42)
///     }
/// }
/// ```
///
/// # Attribute arguments
///
/// - `#[world(init = path::to::fn)]`
///
///   Path to a function to be used for a [`World`] instance construction.
///   Specified function can be either sync or `async`, and either fallible
///   (return [`Result`]) or infallible (return [`World`] itself). In case no
///   function is specified, the [`Default::default()`] will be used for
///   construction.
#[proc_macro_derive(World, attributes(world))]
pub fn world(input: TokenStream) -> TokenStream {
    world::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// In addition to [default parameters] of [Cucumber Expressions], you may
/// implement and use custom ones.
///
/// # Example
///
/// ```rust
/// # use std::{convert::Infallible};
/// #
/// use cucumber::{given, when, Parameter, World};
/// use derive_more::{Deref, FromStr};
///
/// #[derive(Debug, Default, World)]
/// struct MyWorld;
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
