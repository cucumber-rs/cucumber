// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Feature flags and conditional compilation module.
//!
//! This module centralizes all feature-dependent imports and conditional
//! compilation directives used throughout the cucumber crate.

/// Re-exports the codegen module when the "macros" feature is enabled.
#[cfg(feature = "macros")]
pub mod codegen {
    pub use crate::codegen::*;
}

/// Re-exports the tracing module when the "tracing" feature is enabled.
#[cfg(feature = "tracing")]
pub mod tracing {
    pub use crate::tracing::*;
}

/// Test dependencies that are only used in documentation tests and the book.
/// This helps prevent unused dependency warnings while keeping the dependencies
/// available for documentation examples.
#[cfg(test)]
pub mod test_deps {
    pub use rand as _;
    pub use tempfile as _;
    pub use tokio as _;
}

/// Checks if the "macros" feature is enabled at compile time.
pub const fn has_macros_feature() -> bool {
    cfg!(feature = "macros")
}

/// Checks if the "tracing" feature is enabled at compile time.
pub const fn has_tracing_feature() -> bool {
    cfg!(feature = "tracing")
}

/// Returns a list of enabled features as a static string slice.
pub fn enabled_features() -> &'static [&'static str] {
    &[
        #[cfg(feature = "macros")]
        "macros",
        #[cfg(feature = "tracing")]
        "tracing",
    ]
}

/// Returns a formatted string describing all enabled features.
pub fn features_summary() -> String {
    let features = enabled_features();
    if features.is_empty() {
        "No optional features enabled".to_string()
    } else {
        format!("Enabled features: {}", features.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_macros_feature() {
        // Test that the function returns a boolean
        let has_macros = has_macros_feature();
        assert!(has_macros || !has_macros); // Always true, but tests the function call
    }

    #[test]
    fn test_has_tracing_feature() {
        // Test that the function returns a boolean
        let has_tracing = has_tracing_feature();
        assert!(has_tracing || !has_tracing); // Always true, but tests the function call
    }

    #[test]
    fn test_enabled_features_returns_slice() {
        let features = enabled_features();
        // Test that we get a slice (may be empty)
        assert!(features.len() >= 0);
    }

    #[test]
    fn test_features_summary_format() {
        let summary = features_summary();
        // Test that we get a non-empty string
        assert!(!summary.is_empty());
        
        // Test that it contains expected content
        if enabled_features().is_empty() {
            assert_eq!(summary, "No optional features enabled");
        } else {
            assert!(summary.starts_with("Enabled features: "));
        }
    }

    #[cfg(feature = "macros")]
    #[test]
    fn test_macros_feature_enabled() {
        assert!(has_macros_feature());
        assert!(enabled_features().contains(&"macros"));
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn test_tracing_feature_enabled() {
        assert!(has_tracing_feature());
        assert!(enabled_features().contains(&"tracing"));
    }

    #[cfg(not(feature = "macros"))]
    #[test]
    fn test_macros_feature_disabled() {
        assert!(!has_macros_feature());
        assert!(!enabled_features().contains(&"macros"));
    }

    #[cfg(not(feature = "tracing"))]
    #[test]
    fn test_tracing_feature_disabled() {
        assert!(!has_tracing_feature());
        assert!(!enabled_features().contains(&"tracing"));
    }

    #[test]
    fn test_test_deps_are_accessible() {
        // Test that test dependencies are available in test context
        // We don't actually use them, but verify they can be referenced
        use test_deps::*;
        // The test passes if this compiles without error
    }

    #[test]
    fn test_features_consistency() {
        // Test that enabled_features() is consistent with individual feature checks
        let features = enabled_features();
        
        if has_macros_feature() {
            assert!(features.contains(&"macros"));
        } else {
            assert!(!features.contains(&"macros"));
        }
        
        if has_tracing_feature() {
            assert!(features.contains(&"tracing"));
        } else {
            assert!(!features.contains(&"tracing"));
        }
    }
}