// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Definitions for a [`Collection`] which is used to store [`Step`] [`Fn`]s and
//! corresponding [`Regex`] patterns.
//!
//! This module has been refactored into smaller, focused modules while maintaining
//! backward compatibility through re-exports. Each module follows the Single 
//! Responsibility Principle:
//!
//! - [`collection`]: Step collection management and matching
//! - [`context`]: Step execution context and capture handling
//! - [`error`]: Error types for step matching failures
//! - [`location`]: File location tracking for step definitions
//! - [`regex`]: Hashable regex wrapper utilities
//!
//! [`Step`]: gherkin::Step

pub mod collection;
pub mod context;
pub mod error;
pub mod location;
pub mod regex;

// Re-export all public items for easy access
pub use collection::{Collection, WithContext};
pub use context::{CaptureName, Context};
pub use error::AmbiguousMatchError;
pub use location::Location;
pub use regex::HashableRegex;

// Type aliases that depend on other modules
use futures::future::LocalBoxFuture;

/// Alias for a [`gherkin::Step`] function that returns a [`LocalBoxFuture`].
pub type Step<World> =
    for<'a> fn(&'a mut World, Context) -> LocalBoxFuture<'a, ()>;