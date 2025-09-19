//! Top-level [Cucumber] executor.
//!
//! [Cucumber]: https://cucumber.io

// Core module must be first
pub mod core;

// Feature modules
mod cli;
mod clone_impl;
mod defaults;
mod execution;
mod fail_on_skipped;
mod hooks;
mod parser;
mod repeat;
mod runner;
mod steps;

// Re-export the main type and public API
pub use core::Cucumber;
pub use defaults::DefaultCucumber;