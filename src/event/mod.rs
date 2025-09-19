//! Key occurrences in a lifecycle of [Cucumber] execution.
//!
//! The top-level enum here is [`Cucumber`].
//!
//! Each event enum contains variants indicating what stage of execution
//! [`Runner`] is at, and variants with detailed content about the precise
//! sub-event.
//!
//! [`Runner`]: crate::Runner
//! [Cucumber]: https://cucumber.io

// Core modules
pub mod event_struct;
pub mod retries;
pub mod source;

// Event type modules
pub mod cucumber_events;
pub mod feature_events;
pub mod hook_events;
pub mod rule_events;
pub mod scenario_events;
pub mod step_events;

// Re-export public API
pub use cucumber_events::Cucumber;
pub use event_struct::{Event, Info, Metadata};
pub use feature_events::Feature;
pub use hook_events::{Hook, HookType};
pub use retries::Retries;
pub use rule_events::Rule;
pub use scenario_events::{RetryableScenario, Scenario, ScenarioFinished};
pub use source::Source;
pub use step_events::{Step, StepError};