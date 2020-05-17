#![recursion_limit = "512"]
#![deny(rust_2018_idioms)]

// Re-export Gherkin for the convenience of everybody
pub use gherkin;

mod collection;
mod event;
mod regex;
mod runner;
mod steps;
mod cucumber;

use std::panic::UnwindSafe;
use async_trait::async_trait;

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
