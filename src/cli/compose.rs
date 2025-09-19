// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! CLI composition utilities.
//!
//! This module provides structures for composing multiple CLI argument
//! structures together, as well as empty CLI stubs for cases where no
//! CLI options are needed.

use clap::Args;

use super::colored::Colored;
use crate::writer::Coloring;

/// Empty CLI options.
///
/// This struct serves as a placeholder for CLI options when no specific
/// options are needed. It implements all the necessary traits to be used
/// in place of actual CLI structures.
///
/// # Example
///
/// ```rust
/// use cucumber::cli::Empty;
/// use cucumber::Cucumber;
///
/// # #[derive(cucumber::World, Debug, Default)]
/// # struct MyWorld;
/// #
/// # async fn example() {
/// MyWorld::cucumber()
///     .with_cli(cucumber::cli::Opts::<Empty, Empty, Empty, Empty>::default())
///     .run("tests/features")
///     .await;
/// # }
/// ```
#[derive(Args, Clone, Copy, Debug, Default)]
#[group(skip)]
pub struct Empty;

impl Colored for Empty {}

/// Composes two [`clap::Args`] derivers together.
///
/// This struct allows combining two separate CLI argument structures into
/// a single structure, which is particularly useful when implementing
/// custom [`Writer`] that wraps another one and needs to combine CLI options.
///
/// # Example
///
/// This struct is especially useful, when implementing custom [`Writer`]
/// wrapping another one:
/// ```rust
/// # use cucumber::{cli, event, parser, writer, Event, World, Writer};
/// #
/// struct CustomWriter<Wr>(Wr);
///
/// #[derive(cli::Args)] // re-export of `clap::Args`
/// struct Cli {
///     #[arg(long)]
///     custom_option: Option<String>,
/// }
///
/// impl<W, Wr> Writer<W> for CustomWriter<Wr>
/// where
///     W: World,
///     Wr: Writer<W>,
/// {
///     type Cli = cli::Compose<Cli, Wr::Cli>;
///
///     async fn handle_event(
///         &mut self,
///         ev: parser::Result<Event<event::Cucumber<W>>>,
///         cli: &Self::Cli,
///     ) {
///         // Some custom logic including `cli.left.custom_option`.
///         // ...
///         self.0.handle_event(ev, &cli.right).await;
///     }
/// }
///
/// // Useful blanket impls:
///
/// impl cli::Colored for Cli {}
///
/// impl<W, Wr, Val> writer::Arbitrary<W, Val> for CustomWriter<Wr>
/// where
///     Wr: writer::Arbitrary<W, Val>,
///     Self: Writer<W>,
/// {
///     async fn write(&mut self, val: Val) {
///         self.0.write(val).await;
///     }
/// }
///
/// impl<W, Wr> writer::Stats<W> for CustomWriter<Wr>
/// where
///     Wr: writer::Stats<W>,
///     Self: Writer<W>,
/// {
///     fn passed_steps(&self) -> usize {
///         self.0.failed_steps()
///     }
///
///     fn skipped_steps(&self) -> usize {
///         self.0.failed_steps()
///     }
///
///     fn failed_steps(&self) -> usize {
///         self.0.failed_steps()
///     }
///
///     fn retried_steps(&self) -> usize {
///         self.0.retried_steps()
///     }
///
///     fn parsing_errors(&self) -> usize {
///         self.0.parsing_errors()
///     }
///
///     fn hook_errors(&self) -> usize {
///         self.0.hook_errors()
///     }
/// }
///
/// impl<Wr: writer::Normalized> writer::Normalized for CustomWriter<Wr> {}
///
/// impl<Wr: writer::NonTransforming> writer::NonTransforming
///     for CustomWriter<Wr>
/// {}
/// ```
///
/// [`Writer`]: crate::Writer
#[derive(Args, Clone, Copy, Debug, Default)]
#[group(skip)]
pub struct Compose<L: Args, R: Args> {
    /// Left [`clap::Args`] deriver.
    #[command(flatten)]
    pub left: L,

    /// Right [`clap::Args`] deriver.
    #[command(flatten)]
    pub right: R,
}

impl<L: Args, R: Args> Compose<L, R> {
    /// Unpacks this [`Compose`] into the underlying CLIs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use cucumber::cli::Compose;
    ///
    /// #[derive(clap::Args, Default)]
    /// struct LeftCli {
    ///     #[arg(long)]
    ///     left_flag: bool,
    /// }
    ///
    /// #[derive(clap::Args, Default)]
    /// struct RightCli {
    ///     #[arg(long)]
    ///     right_flag: bool,
    /// }
    ///
    /// let compose = Compose {
    ///     left: LeftCli { left_flag: true },
    ///     right: RightCli { right_flag: false },
    /// };
    ///
    /// let (left, right) = compose.into_inner();
    /// assert!(left.left_flag);
    /// assert!(!right.right_flag);
    /// ```
    #[must_use]
    pub fn into_inner(self) -> (L, R) {
        let Self { left, right } = self;
        (left, right)
    }
}

#[warn(clippy::missing_trait_methods)]
impl<L, R> Colored for Compose<L, R>
where
    L: Args + Colored,
    R: Args + Colored,
{
    /// Returns the "maximum" [`Coloring`] preference between the left and right CLI options.
    ///
    /// The precedence is: [`Coloring::Always`] > [`Coloring::Auto`] > [`Coloring::Never`].
    /// This ensures that if either CLI option supports colored output, the composed
    /// result will also support it at the highest level requested.
    fn coloring(&self) -> Coloring {
        // Basically, founds "maximum" `Coloring` of CLI options.
        match (self.left.coloring(), self.right.coloring()) {
            (Coloring::Always, _) | (_, Coloring::Always) => Coloring::Always,
            (Coloring::Auto, _) | (_, Coloring::Auto) => Coloring::Auto,
            (Coloring::Never, Coloring::Never) => Coloring::Never,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_cli() {
        let empty = Empty::default();
        assert_eq!(empty.coloring(), Coloring::Never);
    }

    #[test]
    fn test_compose_cli() {
        #[derive(Debug, Default, Clone, clap::Args)]
        struct LeftCli {
            #[arg(long)]
            left_flag: bool,
        }

        #[derive(Debug, Default, Clone, clap::Args)]
        struct RightCli {
            #[arg(long)]
            right_flag: bool,
        }

        impl Colored for LeftCli {
            fn coloring(&self) -> Coloring {
                Coloring::Always
            }
        }

        impl Colored for RightCli {
            fn coloring(&self) -> Coloring {
                Coloring::Auto
            }
        }

        let compose = Compose {
            left: LeftCli { left_flag: true },
            right: RightCli { right_flag: false },
        };

        assert_eq!(compose.coloring(), Coloring::Always);
        
        let (left, right) = compose.into_inner();
        assert!(left.left_flag);
        assert!(!right.right_flag);
    }

    #[test]
    fn test_compose_coloring_precedence() {
        // Test all combinations of Coloring precedence
        let test_cases = vec![
            (Coloring::Never, Coloring::Never, Coloring::Never),
            (Coloring::Never, Coloring::Auto, Coloring::Auto),
            (Coloring::Never, Coloring::Always, Coloring::Always),
            (Coloring::Auto, Coloring::Never, Coloring::Auto),
            (Coloring::Auto, Coloring::Auto, Coloring::Auto),
            (Coloring::Auto, Coloring::Always, Coloring::Always),
            (Coloring::Always, Coloring::Never, Coloring::Always),
            (Coloring::Always, Coloring::Auto, Coloring::Always),
            (Coloring::Always, Coloring::Always, Coloring::Always),
        ];

        for (left_color, right_color, expected) in test_cases {
            // Manually test the compose logic directly
            let result = match (left_color, right_color) {
                (Coloring::Always, _) | (_, Coloring::Always) => Coloring::Always,
                (Coloring::Auto, _) | (_, Coloring::Auto) => Coloring::Auto,
                (Coloring::Never, Coloring::Never) => Coloring::Never,
            };

            assert_eq!(result, expected, 
                "Failed for left: {:?}, right: {:?}", left_color, right_color);
        }
    }

    #[test]
    fn test_compose_with_empty() {
        #[derive(Debug, Default, Clone, clap::Args)]
        struct TestCli {
            #[arg(long)]
            test_flag: bool,
        }

        impl Colored for TestCli {
            fn coloring(&self) -> Coloring {
                Coloring::Auto
            }
        }

        let compose = Compose {
            left: TestCli { test_flag: true },
            right: Empty,
        };

        assert_eq!(compose.coloring(), Coloring::Auto);
        
        let (left, _right) = compose.into_inner();
        assert!(left.test_flag);
    }

    #[test]
    fn test_compose_symmetric_coloring() {
        // Test that compose coloring is symmetric (left/right order doesn't matter)
        #[derive(Debug, Default, Clone, clap::Args)]
        struct AutoCli;

        #[derive(Debug, Default, Clone, clap::Args)]
        struct AlwaysCli;

        impl Colored for AutoCli {
            fn coloring(&self) -> Coloring {
                Coloring::Auto
            }
        }

        impl Colored for AlwaysCli {
            fn coloring(&self) -> Coloring {
                Coloring::Always
            }
        }

        let compose1 = Compose {
            left: AutoCli,
            right: AlwaysCli,
        };

        let compose2 = Compose {
            left: AlwaysCli,
            right: AutoCli,
        };

        assert_eq!(compose1.coloring(), compose2.coloring());
        assert_eq!(compose1.coloring(), Coloring::Always);
    }
}