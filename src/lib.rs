#![recursion_limit = "512"]
#![deny(rust_2018_idioms)]

// Re-export Gherkin for the convenience of everybody
pub use gherkin;

mod collection;
mod cucumber;
mod event;
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
