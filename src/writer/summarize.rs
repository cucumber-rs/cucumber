//! [`Writer`]-wrapper for collecting a summary of execution.
//!
//! This module has been refactored into focused submodules for better maintainability
//! and testability. The main functionality remains available through re-exports.

// Import the modular implementation
pub mod core;
pub mod formatting;
pub mod stats;
pub mod state;
pub mod tracking;

// Re-export all public types for backward compatibility
pub use self::{
    core::{SkipFn, Summarizable, Summarize},
    formatting::SummaryFormatter,
    stats::Stats,
    state::State,
    tracking::{HandledScenarios, Indicator},
};

// Also re-export the formatting functionality for Styles
use crate::writer::out::Styles;

// TODO: Try remove on next Rust version update.
#[expect(clippy::allow_attributes, reason = "`#[expect]` doesn't work here")]
#[allow( // intentional
    clippy::multiple_inherent_impl,
    reason = "related to summarization only"
)]
impl Styles {
    /// Generates a formatted summary [`String`].
    #[must_use]
    pub fn summary<W>(&self, summary: &Summarize<W>) -> String {
        SummaryFormatter::summary(self, summary)
    }

    /// Formats [`Stats`] for a terminal output.
    #[must_use]
    pub fn format_stats(&self, stats: Stats) -> std::borrow::Cow<'static, str> {
        SummaryFormatter::format_stats(self, stats)
    }

    /// Adds `s` to `singular` if the given `num` is not `1`.
    fn maybe_plural(
        &self,
        singular: impl Into<std::borrow::Cow<'static, str>>,
        num: usize,
    ) -> std::borrow::Cow<'static, str> {
        SummaryFormatter::maybe_plural(self, singular, num)
    }
}