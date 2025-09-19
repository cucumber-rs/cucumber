//! Common test utilities for the cucumber crate.
//!
//! This module provides reusable test structures and implementations
//! that are used across multiple test modules.

#[cfg(test)]
pub mod common {
    use std::future::Future;

    /// Empty CLI implementation for tests that don't need CLI arguments.
    #[derive(Debug, Default, Clone)]
    pub struct EmptyCli;

    impl clap::FromArgMatches for EmptyCli {
        fn from_arg_matches(_matches: &clap::ArgMatches) -> clap::error::Result<Self> {
            Ok(Self)
        }
        fn update_from_arg_matches(&mut self, _matches: &clap::ArgMatches) -> clap::error::Result<()> {
            Ok(())
        }
    }

    impl clap::Args for EmptyCli {
        fn augment_args(cmd: clap::Command) -> clap::Command {
            cmd
        }
        fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
            cmd
        }
    }

    /// Standard test world implementation.
    #[derive(Debug, Default, Clone)]
    pub struct TestWorld;

    impl crate::World for TestWorld {
        type Error = std::convert::Infallible;

        async fn new() -> Result<Self, Self::Error> {
            Ok(Self)
        }
    }
}