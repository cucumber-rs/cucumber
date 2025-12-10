//! Common types and traits for external integrations

// Re-export core event types
pub use crate::event::{
    self, 
    Cucumber as CucumberEvent, 
    Feature as FeatureEvent, 
    Scenario as ScenarioEvent,
    Step as StepEvent,
    Source,
    RetryableScenario,
};

// Re-export observer types when available
#[cfg(feature = "observability")]
pub use crate::observer::{
    TestObserver, 
    ObserverRegistry, 
    ObservationContext
};

// Re-export runner types
pub use crate::runner::Basic as BasicRunner;

// Re-export writer types
pub use crate::writer::{
    Writer,
    Basic as BasicWriter,
};

// Re-export World trait
pub use crate::World;

// Re-export Event wrapper
pub use crate::Event;