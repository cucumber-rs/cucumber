// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
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
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::decimal_literal_representation,
    clippy::else_if_without_else,
    clippy::empty_line_after_outer_attr,
    clippy::exit,
    clippy::expect_used,
    clippy::fallible_impl_from,
    clippy::filetype_is_file,
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
    clippy::verbose_file_reads,
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
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod cli;
mod cucumber;
pub mod event;
pub mod expression;
pub mod feature;
pub mod parser;
pub mod runner;
pub mod step;
pub mod tag;
pub mod writer;

#[cfg(feature = "macros")]
pub mod codegen;

use std::error::Error as StdError;

use async_trait::async_trait;

pub use gherkin;

#[cfg(feature = "macros")]
#[doc(inline)]
pub use self::codegen::WorldInit;
#[cfg(feature = "macros")]
#[doc(inline)]
pub use cucumber_codegen::{given, then, when, WorldInit};

#[doc(inline)]
pub use self::{
    cucumber::Cucumber,
    event::Event,
    expression::Parameter,
    parser::Parser,
    runner::{Runner, ScenarioType},
    step::Step,
    writer::{
        Arbitrary as ArbitraryWriter, Ext as WriterExt,
        Failure as FailureWriter, Writer,
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
/// [0]: https://cucumber.io/docs/gherkin/reference/#descriptions
/// [1]: https://docs.rs/once_cell
/// [2]: https://doc.rust-lang.org/book/ch16-03-shared-state.html
/// [Cucumber]: https://cucumber.io
#[async_trait(?Send)]
pub trait World: Sized + 'static {
    /// Error of creating a new [`World`] instance.
    type Error: StdError;

    /// Creates a new [`World`] instance.
    async fn new() -> Result<Self, Self::Error>;
}
