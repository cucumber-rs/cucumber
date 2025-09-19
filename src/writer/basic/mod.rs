//! Basic CLI [`Writer`] implementation.
//!
//! This writer outputs test results in a human-readable format similar to
//! standard test runners, with colored output support and verbose modes.

mod basic_struct;
mod cli;
mod event_handlers;
mod feature_output;
mod scenario_output;
mod formatting;
mod step_output;
mod background_output;
mod output_formatter;

pub use basic_struct::Basic;
pub use cli::{Cli, Coloring};
pub use formatting::{coerce_error, trim_path};