//! [`tracing`] integration layer.
//!
//! This module provides comprehensive tracing integration for Cucumber tests,
//! allowing for detailed logging and span management during test execution.

// Import the modular implementation
mod tracing;

// Re-export everything for backward compatibility
pub use self::tracing::*;

