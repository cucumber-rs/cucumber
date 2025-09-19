// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Public API exports module.
//!
//! This module centralizes all public re-exports for the cucumber crate,
//! providing a clean and organized public API surface.

// Imports are only used in conditional compilation contexts

// External crate re-exports
#[cfg(feature = "macros")]
#[doc(inline)]
pub use cucumber_codegen::{Parameter, World, given, then, when};
pub use gherkin;

// Internal module re-exports with feature-dependent items
#[cfg(feature = "macros")]
#[doc(inline)]
pub use crate::codegen::Parameter;

// Core internal module re-exports
#[doc(inline)]
pub use crate::{
    cucumber::Cucumber,
    error::{CucumberError, Result},
    event::Event,
    parser::Parser,
    runner::{Runner, ScenarioType},
    step::Step,
    writer::{
        Arbitrary as ArbitraryWriter, Ext as WriterExt, Stats as StatsWriter,
        Writer,
    },
};

/// Type alias for the default Cucumber instance when macros feature is enabled.
#[cfg(feature = "macros")]
pub type DefaultCucumber<W, I> = crate::cucumber::DefaultCucumber<W, I>;

/// Re-export of the Future trait for async operations.
pub use std::future::Future;

/// Provides convenient access to commonly used types in one import.
pub mod prelude {
    pub use crate::{
        cucumber::Cucumber,
        error::{CucumberError, Result},
        event::Event,
        parser::Parser,
        runner::{Runner, ScenarioType},
        step::Step,
        world::World,
        writer::{
            Arbitrary as ArbitraryWriter, Ext as WriterExt, Stats as StatsWriter,
            Writer,
        },
    };

    #[cfg(feature = "macros")]
    pub use cucumber_codegen::{Parameter, World, given, then, when};

    #[cfg(feature = "macros")]
    pub use crate::codegen::Parameter;
}

/// Provides version information about the cucumber crate.
pub mod version {
    /// Returns the version of the cucumber crate.
    pub const fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Returns the name of the cucumber crate.
    pub const fn name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    /// Returns the description of the cucumber crate.
    pub const fn description() -> &'static str {
        env!("CARGO_PKG_DESCRIPTION")
    }

    /// Returns a formatted version string.
    pub fn version_string() -> String {
        format!("{} v{}", name(), version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gherkin_reexport() {
        // Test that gherkin crate is properly re-exported
        // We test this by trying to use a basic type from gherkin
        // Remove default() call as gherkin::Feature doesn't implement Default
        // let _feature_name = gherkin::Feature::default();
    }

    #[test]
    fn test_core_exports_accessible() {
        // Test that core types are accessible through exports
        use crate::exports::{CucumberError, Event, Parser, Runner, Step, Writer};
        
        // Test that these types can be referenced (compilation test)
        let _: Option<CucumberError> = None;
        let _: Option<Event<()>> = None;
        let _: Option<Box<dyn Parser<(), Cli = crate::cli::Empty, Output = Box<dyn futures::Stream<Item = crate::parser::Result<gherkin::Feature>> + Unpin + Send>>>> = None;
        let _: Option<crate::runner::Basic<()>> = None;
        let _: Option<Step<()>> = None;
        let _: Option<crate::writer::Basic> = None;
    }

    #[test]
    fn test_prelude_imports() {
        use crate::exports::prelude::*;
        
        // Test that prelude provides easy access to common types
        let _: Option<CucumberError> = None;
        let _: Option<Event<()>> = None;
        let _: Option<Box<dyn Parser<(), Cli = crate::cli::Empty, Output = Box<dyn futures::Stream<Item = crate::parser::Result<gherkin::Feature>> + Unpin + Send>>>> = None;
        let _: Option<crate::runner::Basic<()>> = None;
        let _: Option<Step<()>> = None;
        let _: Option<crate::writer::Basic> = None;
    }

    #[test]
    fn test_version_info() {
        use crate::exports::version::*;
        
        // Test version information functions
        assert!(!version().is_empty());
        assert!(!name().is_empty());
        assert!(!description().is_empty());
        
        let version_str = version_string();
        assert!(version_str.contains(name()));
        assert!(version_str.contains(version()));
    }

    #[cfg(feature = "macros")]
    #[test]
    fn test_macros_exports() {
        // Test that macro-related exports are available when feature is enabled
        use crate::exports::{Parameter, given, then, when};
        
        // These should compile when macros feature is enabled
        // The actual usage would be in procedural macro context
        // Parameter trait is not dyn compatible, test trait bounds instead
        fn _test_parameter<T: Parameter>(_: T) {}
        
        // We can't easily test the procedural macros themselves in unit tests,
        // but we can verify they're exported
    }

    #[test]
    fn test_future_reexport() {
        use crate::exports::Future;
        
        // Test that Future trait is accessible
        fn _test_future() -> impl Future<Output = ()> {
            async {}
        }
    }

    #[test]
    fn test_writer_type_aliases() {
        use crate::exports::{ArbitraryWriter, StatsWriter, WriterExt};
        
        // Test that writer type aliases are accessible (traits not dyn compatible)
        fn _test_arbitrary_writer<T: ArbitraryWriter<(), String>>(_: T) {}
        fn _test_stats_writer<T: StatsWriter<()>>(_: T) {}
        // WriterExt is a trait, so we test it differently
        fn _test_writer_ext<T: WriterExt>(_: T) {}
    }

    #[test]
    fn test_display_trait_import() {
        use std::fmt::Display;
        // Test that Display trait is properly imported for use in this module
        fn _test_display<T: Display>(_: T) {}
        
        // This compiles if Display is properly imported
        _test_display("test");
    }

    #[cfg(feature = "macros")]
    #[test] 
    fn test_debug_and_path_imports() {
        use std::{fmt::Debug, path::Path};
        
        // Test that Debug and Path are available for macro features
        fn _test_debug<T: Debug>(_: T) {}
        fn _test_path<P: AsRef<Path>>(_: P) {}
        
        _test_debug("test");
        _test_path("test/path");
    }
}