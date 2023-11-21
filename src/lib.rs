// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
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
// TODO: Remove on next `derive_more` major version.
#![allow(clippy::uninlined_format_args)]
// TODO: Massive false positives on `.await` points. Try remove on next Rust
//       version.
#![allow(clippy::multiple_unsafe_ops_per_block)]

pub mod cli;
mod cucumber;
pub mod event;
pub mod feature;
pub(crate) mod future;
pub mod parser;
pub mod runner;
pub mod step;
pub mod tag;
pub mod writer;

#[cfg(feature = "macros")]
pub mod codegen;
#[cfg(feature = "tracing")]
pub mod tracing;

// TODO: Remove once tests run without complains about it.
#[cfg(test)]
mod actually_used_crates_in_tests_and_book {
    use rand as _;
    use tempfile as _;
    use tokio as _;
}

use std::fmt::Display;
#[cfg(feature = "macros")]
use std::{fmt::Debug, path::Path};

use async_trait::async_trait;

#[cfg(feature = "macros")]
use self::{
    codegen::{StepConstructor as _, WorldInventory},
    cucumber::DefaultCucumber,
};

pub use gherkin;

#[cfg(feature = "macros")]
#[doc(inline)]
pub use self::codegen::Parameter;
#[cfg(feature = "macros")]
#[doc(inline)]
pub use cucumber_codegen::{given, then, when, Parameter, World};

#[doc(inline)]
pub use self::{
    cucumber::Cucumber,
    event::Event,
    parser::Parser,
    runner::{Runner, ScenarioType},
    step::Step,
    writer::{
        Arbitrary as ArbitraryWriter, Ext as WriterExt, Stats as StatsWriter,
        Writer,
    },
};

/// Represents a shared user-defined state for a [Cucumber] run.
/// It lives on per-[scenario][0] basis.
///
/// This crate doesn't provide out-of-box solution for managing state shared
/// across [scenarios][0], because we want some friction there to avoid tests
/// being dependent on each other. If your workflow needs a way to share state
/// between [scenarios][0] (ex. database connection pool), we recommend using
/// [`once_cell`][1] crate or organize it other way via [shared state][2].
///
/// [0]: https://cucumber.io/docs/gherkin/reference#descriptions
/// [1]: https://docs.rs/once_cell
/// [2]: https://doc.rust-lang.org/book/ch16-03-shared-state.html
/// [Cucumber]: https://cucumber.io
#[async_trait(?Send)]
pub trait World: Sized + 'static {
    /// Error of creating a new [`World`] instance.
    type Error: Display;

    /// Creates a new [`World`] instance.
    async fn new() -> Result<Self, Self::Error>;

    #[cfg(feature = "macros")]
    /// Returns runner for tests with auto-wired steps marked by [`given`],
    /// [`when`] and [`then`] attributes.
    #[must_use]
    fn collection() -> step::Collection<Self>
    where
        Self: Debug + WorldInventory,
    {
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

    #[cfg(feature = "macros")]
    /// Returns default [`Cucumber`] with all the auto-wired [`Step`]s.
    #[must_use]
    fn cucumber<I: AsRef<Path>>() -> DefaultCucumber<Self, I>
    where
        Self: Debug + WorldInventory,
    {
        Cucumber::new().steps(Self::collection())
    }

    #[cfg(feature = "macros")]
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
    async fn run<I: AsRef<Path>>(input: I)
    where
        Self: Debug + WorldInventory,
    {
        Self::cucumber().run_and_exit(input).await;
    }

    #[cfg(feature = "macros")]
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
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    async fn filter_run<I, F>(input: I, filter: F)
    where
        Self: Debug + WorldInventory,
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
