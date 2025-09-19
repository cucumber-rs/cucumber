// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for outputting [`Cucumber`] events.
//!
//! This module provides various writers for different output formats, along with
//! consolidation utilities in the [`common`] module that reduce code duplication
//! and provide shared functionality across different writer implementations.
//!
//! # Writer Consolidation
//! 
//! The [`common`] module provides:
//! - [`StepContext`] and [`ScenarioContext`] to consolidate commonly-passed parameters
//! - [`WriterStats`] for standardized statistics tracking
//! - [`OutputFormatter`] trait for common output operations with proper error handling
//! - Helper utilities for world formatting, error handling, and context management
//!
//! # Architecture
//!
//! The writer module is organized into several focused sub-modules:
//!
//! - [`traits`]: Core traits defining writer behavior ([`Writer`], [`Arbitrary`], [`Stats`])
//! - [`ext`]: Extension trait for fluent writer composition and transformations
//! - [`types`]: Common types and marker traits ([`Verbosity`], [`NonTransforming`])
//! - Individual writer implementations: [`basic`], [`json`], [`junit`], etc.
//! - Writer combinators: [`normalize`], [`summarize`], [`repeat`], [`tee`], etc.
//!
//! [`Cucumber`]: crate::event::Cucumber
//! [`StepContext`]: common::StepContext
//! [`ScenarioContext`]: common::ScenarioContext
//! [`WriterStats`]: common::WriterStats
//! [`OutputFormatter`]: common::OutputFormatter

// Core modules - new modular structure
pub mod ext;
pub mod traits;
pub mod types;

// Writer implementations
pub mod basic;
pub mod common;
pub mod discard;
pub mod fail_on_skipped;
#[cfg(feature = "output-json")]
pub mod json;
#[cfg(feature = "output-junit")]
pub mod junit;
#[cfg(feature = "libtest")]
pub mod libtest;
pub mod normalize;
pub mod or;
pub mod out;
pub mod repeat;
pub mod summarize;
pub mod tee;

// Re-export core traits and types for backward compatibility
#[doc(inline)]
pub use self::{
    traits::{Arbitrary, Stats, Writer},
    ext::Ext,
    types::{NonTransforming, Verbosity},
};

// Re-export specific writer implementations
#[cfg(feature = "output-json")]
#[doc(inline)]
pub use self::json::Json;
#[cfg(feature = "output-junit")]
#[doc(inline)]
pub use self::junit::JUnit;
#[cfg(feature = "libtest")]
#[doc(inline)]
pub use self::libtest::Libtest;

// Re-export writer utilities and combinators
#[doc(inline)]
pub use self::{
    basic::{Basic, Coloring},
    common::{
        StepContext, ScenarioContext, WriterStats, OutputFormatter,
        WorldFormatter, ErrorFormatter, WriterExt as CommonWriterExt,
    },
    fail_on_skipped::FailOnSkipped,
    normalize::{AssertNormalized, Normalize, Normalized},
    or::Or,
    repeat::Repeat,
    summarize::{Summarizable, Summarize},
    tee::Tee,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_reexports_exist() {
        // Test that core traits are available
        use self::{Arbitrary, Stats, Writer, Ext, NonTransforming, Verbosity};
        
        // Test that writer implementations are available
        use self::{Basic, Coloring, FailOnSkipped, AssertNormalized, Normalize, 
                  Normalized, Or, Repeat, Summarize, Tee};
        
        // Test that common utilities are available
        use self::{StepContext, ScenarioContext, WriterStats, OutputFormatter,
                  WorldFormatter, ErrorFormatter, CommonWriterExt};
        
        // Verify types work as expected
        let _verbosity = Verbosity::Default;
        assert_eq!(_verbosity as u8, 0);
    }

    #[cfg(feature = "output-json")]
    #[test]
    fn test_json_writer_available() {
        use self::Json;
        // Just test that the type is accessible
    }

    #[cfg(feature = "output-junit")]
    #[test]
    fn test_junit_writer_available() {
        use self::JUnit;
        // Just test that the type is accessible
    }

    #[cfg(feature = "libtest")]
    #[test]
    fn test_libtest_writer_available() {
        use self::Libtest;
        // Just test that the type is accessible
    }

    #[test]
    fn test_backward_compatibility_imports() {
        // Verify all the public items from the original mod.rs are still available
        // This ensures we don't break existing code that depends on these exports
        
        // Writer utilities - just check they're importable
        use self::{Basic, FailOnSkipped, Normalize, Or, Repeat, Summarize, Tee};
        
        // Common types
        use self::{Verbosity, NonTransforming};
        
        // Test verbosity enum works
        let verbosity = Verbosity::Default;
        assert!(!verbosity.shows_world());
        
        // This test mainly serves as a compile-time check
    }
}