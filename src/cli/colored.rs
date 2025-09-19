// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Coloring support for CLI options.
//!
//! This module provides the [`Colored`] trait that allows CLI structures
//! to indicate their support for colored output, which is essential for
//! terminal-based output formatting.

use crate::writer::Coloring;

/// Indication whether a [`Writer`] using CLI options supports colored output.
///
/// This trait allows CLI options to specify their coloring preferences,
/// which can then be used by writers to determine whether to output
/// colored text or plain text.
///
/// [`Writer`]: crate::Writer
pub trait Colored {
    /// Returns [`Coloring`] indicating whether a [`Writer`] using CLI options
    /// supports colored output or not.
    ///
    /// # Default Implementation
    ///
    /// The default implementation returns [`Coloring::Never`], which means
    /// no colored output is supported. This is a safe default for CLI
    /// options that don't have specific coloring requirements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cucumber::cli::Colored;
    /// use cucumber::writer::Coloring;
    ///
    /// #[derive(clap::Args)]
    /// struct MyCli {
    ///     #[arg(long)]
    ///     force_color: bool,
    /// }
    ///
    /// impl Colored for MyCli {
    ///     fn coloring(&self) -> Coloring {
    ///         if self.force_color {
    ///             Coloring::Always
    ///         } else {
    ///             Coloring::Auto
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// [`Writer`]: crate::Writer
    #[must_use]
    fn coloring(&self) -> Coloring {
        Coloring::Never
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Args;

    #[derive(Debug, Default, Clone, Args)]
    struct TestCliNever;

    impl Colored for TestCliNever {}

    #[derive(Debug, Default, Clone, Args)]
    struct TestCliAlways;

    impl Colored for TestCliAlways {
        fn coloring(&self) -> Coloring {
            Coloring::Always
        }
    }

    #[derive(Debug, Default, Clone, Args)]
    struct TestCliAuto;

    impl Colored for TestCliAuto {
        fn coloring(&self) -> Coloring {
            Coloring::Auto
        }
    }

    #[derive(Debug, Default, Clone, Args)]
    struct TestCliConditional {
        #[arg(long)]
        force_color: bool,
    }

    impl Colored for TestCliConditional {
        fn coloring(&self) -> Coloring {
            if self.force_color {
                Coloring::Always
            } else {
                Coloring::Auto
            }
        }
    }

    #[test]
    fn test_default_coloring() {
        let cli = TestCliNever::default();
        assert_eq!(cli.coloring(), Coloring::Never);
    }

    #[test]
    fn test_always_coloring() {
        let cli = TestCliAlways::default();
        assert_eq!(cli.coloring(), Coloring::Always);
    }

    #[test]
    fn test_auto_coloring() {
        let cli = TestCliAuto::default();
        assert_eq!(cli.coloring(), Coloring::Auto);
    }

    #[test]
    fn test_conditional_coloring() {
        let cli_false = TestCliConditional { force_color: false };
        assert_eq!(cli_false.coloring(), Coloring::Auto);

        let cli_true = TestCliConditional { force_color: true };
        assert_eq!(cli_true.coloring(), Coloring::Always);
    }

    #[test]
    fn test_coloring_enum_equality() {
        // Test that Coloring enum values can be compared correctly
        assert_eq!(Coloring::Never, Coloring::Never);
        assert_eq!(Coloring::Auto, Coloring::Auto);
        assert_eq!(Coloring::Always, Coloring::Always);
        
        assert_ne!(Coloring::Never, Coloring::Auto);
        assert_ne!(Coloring::Auto, Coloring::Always);
        assert_ne!(Coloring::Never, Coloring::Always);
    }
}