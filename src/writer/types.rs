// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Common types and marker traits for writer implementations.
//!
//! This module contains utility types like [`Verbosity`] levels and marker
//! traits like [`NonTransforming`] that help organize and constrain writer
//! behavior in the pipeline.


/// Marker indicating that a [`Writer`] doesn't transform or rearrange events.
///
/// It's used to ensure that a [`Writer`]s pipeline is built in the right order,
/// avoiding situations like an event transformation isn't done before it's
/// [`Repeat`]ed.
///
/// # Example
///
/// If you want to pipeline [`FailOnSkipped`], [`Summarize`] and [`Repeat`]
/// [`Writer`]s, the code won't compile because of the wrong pipelining order.
///
/// ```rust,compile_fail
/// # use cucumber::{writer, World, WriterExt as _};
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout()
///             .fail_on_skipped() // Fails as `Repeat` will re-output skipped
///             .repeat_failed()   // steps instead of failed ones.
///             .summarized()
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # }
/// ```
///
/// ```rust,compile_fail
/// # use cucumber::{writer, World, WriterExt as _};
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout()
///             .repeat_failed()
///             .fail_on_skipped() // Fails as `Summarize` will count skipped
///             .summarized()      // steps instead of `failed` ones.
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # }
/// ```
///
/// ```rust
/// # use std::panic::AssertUnwindSafe;
/// #
/// # use cucumber::{writer, World, WriterExt as _};
/// # use futures::FutureExt as _;
/// #
/// # #[derive(Debug, Default, World)]
/// # struct MyWorld;
/// #
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// # let fut = async {
/// MyWorld::cucumber()
///     .with_writer(
///         // `Writer`s pipeline is constructed in a reversed order.
///         writer::Basic::stdout() // And, finally, print them.
///             .repeat_failed()    // Then, repeat failed ones once again.
///             .summarized()       // Only then, count summary for them.
///             .fail_on_skipped(), // First, transform skipped steps to failed.
///     )
///     .run_and_exit("tests/features/readme")
///     .await;
/// # };
/// # let err = AssertUnwindSafe(fut)
/// #     .catch_unwind()
/// #     .await
/// #     .expect_err("should err");
/// # let err = err.downcast_ref::<String>().unwrap();
/// # assert_eq!(err, "1 step failed");
/// # }
/// ```
///
/// [`Failed`]: event::Step::Failed
/// [`FailOnSkipped`]: super::FailOnSkipped
/// [`Repeat`]: super::Repeat
/// [`Skipped`]: event::Step::Skipped
/// [`Summarize`]: super::Summarize
/// [`Writer`]: super::Writer
pub trait NonTransforming {}

/// Standard verbosity levels of a [`Writer`].
///
/// [`Writer`]: super::Writer
#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)]
pub enum Verbosity {
    /// None additional info.
    #[default]
    Default = 0,

    /// Outputs the whole [`World`] on [`Failed`] [`Step`]s whenever is
    /// possible.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    /// [`World`]: crate::World
    ShowWorld = 1,

    /// Additionally to [`Verbosity::ShowWorld`] outputs [Doc Strings].
    ///
    /// [Doc Strings]: https://cucumber.io/docs/gherkin/reference#doc-strings
    ShowWorldAndDocString = 2,
}

impl From<u8> for Verbosity {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Default,
            1 => Self::ShowWorld,
            _ => Self::ShowWorldAndDocString,
        }
    }
}

impl From<Verbosity> for u8 {
    fn from(v: Verbosity) -> Self {
        match v {
            Verbosity::Default => 0,
            Verbosity::ShowWorld => 1,
            Verbosity::ShowWorldAndDocString => 2,
        }
    }
}

impl Verbosity {
    /// Indicates whether [`World`] should be outputted on [`Failed`] [`Step`]s
    /// implying this [`Verbosity`].
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    /// [`World`]: crate::World
    #[must_use]
    pub const fn shows_world(&self) -> bool {
        matches!(self, Self::ShowWorld | Self::ShowWorldAndDocString)
    }

    /// Indicates whether [`Step::docstring`]s should be outputted implying this
    /// [`Verbosity`].
    ///
    /// [`Step::docstring`]: gherkin::Step::docstring
    #[must_use]
    pub const fn shows_docstring(&self) -> bool {
        matches!(self, Self::ShowWorldAndDocString)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_conversions() {
        assert_eq!(Verbosity::default() as u8, 0);
        assert_eq!(u8::from(Verbosity::ShowWorld), 1);
        assert_eq!(u8::from(Verbosity::ShowWorldAndDocString), 2);
        
        assert!(matches!(Verbosity::from(0), Verbosity::Default));
        assert!(matches!(Verbosity::from(1), Verbosity::ShowWorld));
        assert!(matches!(Verbosity::from(2), Verbosity::ShowWorldAndDocString));
        assert!(matches!(Verbosity::from(255), Verbosity::ShowWorldAndDocString));
    }

    #[test]
    fn test_verbosity_flags() {
        assert!(!Verbosity::Default.shows_world());
        assert!(!Verbosity::Default.shows_docstring());
        
        assert!(Verbosity::ShowWorld.shows_world());
        assert!(!Verbosity::ShowWorld.shows_docstring());
        
        assert!(Verbosity::ShowWorldAndDocString.shows_world());
        assert!(Verbosity::ShowWorldAndDocString.shows_docstring());
    }

    #[test]
    fn test_verbosity_enum_values() {
        assert_eq!(Verbosity::Default as u8, 0);
        assert_eq!(Verbosity::ShowWorld as u8, 1);
        assert_eq!(Verbosity::ShowWorldAndDocString as u8, 2);
    }

    #[test]
    fn test_verbosity_copy_clone() {
        let v1 = Verbosity::ShowWorld;
        let v2 = v1; // Copy
        let v3 = v1.clone(); // Clone
        
        assert!(matches!(v1, Verbosity::ShowWorld));
        assert!(matches!(v2, Verbosity::ShowWorld));
        assert!(matches!(v3, Verbosity::ShowWorld));
    }

    // Test that NonTransforming is a marker trait
    struct TestWriter;
    
    impl NonTransforming for TestWriter {}

    #[test]
    fn test_non_transforming_marker_trait() {
        // Just verify the trait can be implemented
        let _writer = TestWriter;
    }
}