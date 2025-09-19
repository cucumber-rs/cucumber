// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [JUnit XML report][1] [`Writer`] implementation.
//!
//! This module has been refactored into a modular structure following the 
//! Single Responsibility Principle. The implementation is now split across
//! focused modules in the `junit/` directory:
//!
//! - `cli`: CLI configuration and argument parsing
//! - `error_handler`: Error handling for parser and expansion errors  
//! - `event_handlers`: Event processing logic for different Cucumber events
//! - `test_case_builder`: Test case creation from scenario events
//! - `writer`: Main JUnit writer implementation
//!
//! All original functionality is preserved through re-exports for backward compatibility.
//!
//! [1]: https://llg.cubic.org/docs/junit

// Re-export everything from the modular implementation
mod junit;

pub use junit::{Cli, JUnit};
