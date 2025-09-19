// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Common writer functionality and utilities.
//!
//! This module provides shared functionality used across different writer implementations,
//! organized into focused modules following the Single Responsibility Principle:
//!
//! - [`context`]: Context structures for step and scenario operations
//! - [`stats`]: Statistics tracking for test execution
//! - [`formatting`]: Output formatting utilities and traits
//! - [`formatters`]: Specialized formatters for world and error output
//! - [`extensions`]: Extension traits for common operations
//!
//! All public items are re-exported at the module level for backward compatibility.

pub mod context;
pub mod extensions;
pub mod formatters;
pub mod formatting;
pub mod stats;

// Re-export all public items for backward compatibility
pub use context::{ScenarioContext, StepContext};
pub use extensions::WriterExt;
pub use formatters::{ErrorFormatter, WorldFormatter};
pub use formatting::OutputFormatter;
pub use stats::WriterStats;