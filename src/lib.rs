//! A library implementing the Cucumber testing framework for Rust, in Rust.

#![allow(clippy::module_name_repetitions)]
#![deny(
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code
)]
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

pub mod cucumber;
pub mod event;
pub mod feature;
pub mod parser;
pub mod runner;
pub mod step;
pub mod writer;

use std::error::Error as StdError;

use async_trait::async_trait;

#[doc(inline)]
pub use self::{
    cucumber::Cucumber,
    parser::Parser,
    runner::Runner,
    step::Step,
    writer::{Writer, WriterExt},
};

/// The [`World`] trait represents shared user-defined state
/// for a cucumber run.
#[async_trait(?Send)]
pub trait World: Sized + 'static {
    /// Error of creating [`World`] instance.
    type Error: StdError;

    /// Creates new [`World`] instance.
    async fn new() -> Result<Self, Self::Error>;
}
