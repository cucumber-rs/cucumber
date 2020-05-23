// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![recursion_limit = "512"]
#![deny(rust_2018_idioms)]

// Re-export Gherkin for the convenience of everybody
pub use gherkin;

#[macro_use]
mod macros;

mod collection;
mod cucumber;
mod event;
mod output;
mod regex;
mod runner;
mod steps;

use async_trait::async_trait;
use std::panic::UnwindSafe;

pub use cucumber::Cucumber;
pub use steps::Steps;

const TEST_SKIPPED: &str = "Cucumber: test skipped";

#[macro_export]
macro_rules! skip {
    () => {
        panic!("Cucumber: test skipped");
    };
}

#[async_trait(?Send)]
pub trait World: Sized + UnwindSafe + 'static {
    async fn new() -> Self;
}

pub trait EventHandler: 'static {
    fn handle_event(&mut self, event: event::CucumberEvent);
}
