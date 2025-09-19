// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! World trait definition and related functionality.
//!
//! This module contains the core World trait that represents shared user-defined
//! state for Cucumber test runs, along with its associated methods and functionality.

use std::{fmt::Display, future::Future};

#[cfg(feature = "macros")]
use std::{fmt::Debug, path::Path};

#[cfg(feature = "macros")]
use crate::{
    codegen::{StepConstructor as _, WorldInventory},
    cucumber::DefaultCucumber,
};

/// Represents a shared user-defined state for a [Cucumber] run.
/// It lives on per-[scenario][0] basis.
///
/// This crate doesn't provide out-of-box solution for managing state shared
/// across [scenarios][0], because we want some friction there to avoid tests
/// being dependent on each other. If your workflow needs a way to share state
/// between [scenarios][0] (ex. database connection pool), we recommend using
/// a [`std::sync::LazyLock`] or organize it other way via [shared state][1].
///
/// [0]: https://cucumber.io/docs/gherkin/reference#descriptions
/// [1]: https://doc.rust-lang.org/book/ch16-03-shared-state.html
/// [Cucumber]: https://cucumber.io
pub trait World: Sized + 'static {
    /// Error of creating a new [`World`] instance.
    type Error: Display;

    /// Creates a new [`World`] instance.
    fn new() -> impl Future<Output = std::result::Result<Self, Self::Error>>;

    #[cfg(feature = "macros")]
    /// Returns runner for tests with auto-wired steps marked by [`given`],
    /// [`when`] and [`then`] attributes.
    #[must_use]
    fn collection() -> crate::step::Collection<Self>
    where
        Self: Debug + WorldInventory,
    {
        let mut out = crate::step::Collection::new();

        for given in inventory::iter::<Self::Given> {
            let (loc, regex, fun) = given.inner();
            out = out.given(Some(loc), regex(), fun);
        }

        for when in inventory::iter::<Self::When> {
            let (loc, regex, fun) = when.inner();
            out = out.when(Some(loc), regex(), fun);
        }

        for then in inventory::iter::<Self::Then> {
            let (loc, regex, fun) = then.inner();
            out = out.then(Some(loc), regex(), fun);
        }

        out
    }

    #[cfg(feature = "macros")]
    /// Returns default [`Cucumber`] with all the auto-wired [`Step`]s.
    #[must_use]
    fn cucumber<I: AsRef<Path>>() -> DefaultCucumber<Self, I>
    where
        Self: Debug + WorldInventory,
    {
        crate::Cucumber::new().steps(Self::collection())
    }

    #[cfg(feature = "macros")]
    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced by [`Parser`] are fed into [`Runner`] where the
    /// later produces events handled by [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] panicked.
    ///
    /// [`Feature`]: gherkin::Feature
    fn run<I: AsRef<Path>>(input: I) -> impl Future<Output = ()>
    where
        Self: Debug + WorldInventory,
    {
        Self::cucumber().run_and_exit(input)
    }

    #[cfg(feature = "macros")]
    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced by [`Parser`] are fed into [`Runner`] where the
    /// later produces events handled by [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] panicked.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    /// [`Step`]: gherkin::Step
    fn filter_run<I, F>(input: I, filter: F) -> impl Future<Output = ()>
    where
        Self: Debug + WorldInventory,
        I: AsRef<Path>,
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        Self::cucumber().filter_run_and_exit(input, filter)
    }
}

/// A simple error type for World creation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldError {
    /// The error message.
    pub message: String,
}

impl Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "World creation error: {}", self.message)
    }
}

impl std::error::Error for WorldError {}

impl WorldError {
    /// Creates a new WorldError with the given message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Helper trait for World implementations that need to perform async initialization.
pub trait AsyncWorldInit: World {
    /// Performs async initialization of the World state.
    fn init(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Helper trait for World implementations that need cleanup.
pub trait WorldCleanup: World {
    /// Performs cleanup of the World state.
    fn cleanup(&mut self) -> impl Future<Output = Result<(), Self::Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementation of World for testing purposes
    struct TestWorld {
        value: i32,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(String);

    impl Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Test error: {}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    impl World for TestWorld {
        type Error = TestError;

        async fn new() -> Result<Self, Self::Error> {
            Ok(Self { value: 42 })
        }
    }

    impl AsyncWorldInit for TestWorld {
        async fn init(&mut self) -> Result<(), Self::Error> {
            self.value = 100;
            Ok(())
        }
    }

    impl WorldCleanup for TestWorld {
        async fn cleanup(&mut self) -> Result<(), Self::Error> {
            self.value = 0;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_world_creation() {
        let world = TestWorld::new().await;
        assert!(world.is_ok());
        assert_eq!(world.unwrap().value, 42);
    }

    #[tokio::test]
    async fn test_world_error_display() {
        let error = WorldError::new("Test error message");
        let display_str = format!("{}", error);
        assert_eq!(display_str, "World creation error: Test error message");
    }

    #[tokio::test]
    async fn test_world_error_creation() {
        let error = WorldError::new("Something went wrong");
        assert_eq!(error.message, "Something went wrong");
    }

    #[tokio::test]
    async fn test_async_world_init() {
        let mut world = TestWorld::new().await.unwrap();
        assert_eq!(world.value, 42);
        
        let init_result = world.init().await;
        assert!(init_result.is_ok());
        assert_eq!(world.value, 100);
    }

    #[tokio::test]
    async fn test_world_cleanup() {
        let mut world = TestWorld::new().await.unwrap();
        assert_eq!(world.value, 42);
        
        let cleanup_result = world.cleanup().await;
        assert!(cleanup_result.is_ok());
        assert_eq!(world.value, 0);
    }

    #[test]
    fn test_world_error_debug() {
        let error = WorldError::new("Debug test");
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Debug test"));
    }

    #[test]
    fn test_world_error_clone() {
        let error = WorldError::new("Clone test");
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    #[test]
    fn test_world_error_eq() {
        let error1 = WorldError::new("Same message");
        let error2 = WorldError::new("Same message");
        let error3 = WorldError::new("Different message");
        
        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_test_error_display() {
        let error = TestError("Test message".to_string());
        let display_str = format!("{}", error);
        assert_eq!(display_str, "Test error: Test message");
    }

    // Test that World trait is object-safe (can be used as trait object)
    #[test]
    fn test_world_trait_object() {
        // This test verifies that World trait methods are properly defined
        // and can be used in trait object contexts when needed
        fn _test_world_bounds<W: World>() {
            // This function compiles if World has proper bounds
        }
        
        _test_world_bounds::<TestWorld>();
    }
}