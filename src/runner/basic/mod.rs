//! Basic runner implementation with modular architecture.
//!
//! This module provides the default [`Runner`] implementation that executes
//! scenarios with configurable concurrency, retry logic, and hooks.

mod cli_and_types;
mod basic_struct;
mod runner_impl;
mod execution_engine;
mod executor;
mod scenario_storage;
mod supporting_structures;

// Re-export public APIs for backward compatibility
pub use cli_and_types::{
    Cli, ScenarioType, RetryOptions, RetryOptionsWithDeadline,
    WhichScenarioFn, RetryOptionsFn, BeforeHookFn, AfterHookFn,
};
pub use basic_struct::Basic;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{World, Runner};
    use futures::stream;
    use gherkin::Feature;
    use crate::test_utils::common::TestWorld;

    // Using common TestWorld from test_utils

    #[test]
    fn test_module_re_exports() {
        // Test that all public types are accessible
        let _cli = Cli::default();
        let _basic = Basic::<TestWorld>::default();
        let _scenario_type = ScenarioType::Concurrent;
    }

    #[tokio::test]
    async fn test_basic_runner_integration() {
        let runner = Basic::<TestWorld>::default()
            .max_concurrent_scenarios(1)
            .fail_fast();

        // Test that runner can be created and configured
        assert!(runner.max_concurrent_scenarios == Some(1));
        assert!(runner.fail_fast);
    }

    #[test]
    fn test_scenario_type_enum() {
        use ScenarioType::*;
        
        assert_eq!(Serial, Serial);
        assert_ne!(Serial, Concurrent);
        
        // Test that enum can be pattern matched
        match Concurrent {
            Concurrent => {}
            Serial => panic!("Should be Concurrent"),
        }
    }

    #[test]
    fn test_retry_options_creation() {
        use std::time::Duration;
        use crate::event::Retries;

        let opts = RetryOptions {
            retries: Retries::initial(3),
            after: Some(Duration::from_secs(1)),
        };

        assert_eq!(opts.retries.left, 3);
        assert_eq!(opts.after, Some(Duration::from_secs(1)));
    }
}