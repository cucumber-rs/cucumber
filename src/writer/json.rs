// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Cucumber JSON format][1] [`Writer`] implementation.
//!
//! This module has been refactored into a modular structure for better maintainability
//! and follows the Single Responsibility Principle. All original functionality is
//! preserved through re-exports.
//!
//! [1]: https://github.com/cucumber/cucumber-json-schema

// Import all modules
mod element;
mod feature;
mod handlers;
mod types;
mod writer;

// Re-export all public types for backward compatibility
pub use self::{
    element::Element,
    feature::Feature,
    types::{Base64, Embedding, HookResult, RunResult, Status, Step, Tag},
    writer::Json,
};
