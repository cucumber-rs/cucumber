/// Observer trait for external systems like ObservaBDD
/// 
/// This provides a lightweight integration point for observability
/// without adding runtime overhead when not in use.

use crate::{event, Event, World};

/// Context provided to observers containing execution metadata
#[derive(Clone, Debug)]
pub struct ObservationContext {
    pub scenario_id: Option<u64>,
    pub feature_name: String,
    pub rule_name: Option<String>,
    pub scenario_name: String,
    pub retry_info: Option<event::Retries>,
    pub tags: Vec<String>,
    pub timestamp: std::time::Instant,
}

/// Observer trait for monitoring test execution
pub trait TestObserver<W: World>: Send + Sync {
    /// Called when an event occurs
    fn on_event(&mut self, event: &Event<event::Cucumber<W>>, context: &ObservationContext);
    
    /// Called when execution starts
    fn on_start(&mut self) {}
    
    /// Called when execution completes
    fn on_finish(&mut self) {}
}

/// No-op observer for when observation is disabled
pub struct NullObserver;

impl<W: World> TestObserver<W> for NullObserver {
    fn on_event(&mut self, _: &Event<event::Cucumber<W>>, _: &ObservationContext) {}
}

/// Registry for managing multiple observers
pub struct ObserverRegistry<W> {
    observers: Vec<Box<dyn TestObserver<W>>>,
    enabled: bool,
}

impl<W> ObserverRegistry<W> {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
            enabled: false,
        }
    }
    
    pub fn register(&mut self, observer: Box<dyn TestObserver<W>>) 
    where 
        W: World,
    {
        self.observers.push(observer);
        self.enabled = true;
    }
    
    #[inline]
    pub fn notify(&mut self, event: &Event<event::Cucumber<W>>, context: &ObservationContext)
    where
        W: World,
    {
        if self.enabled {
            for observer in &mut self.observers {
                observer.on_event(event, context);
            }
        }
    }
}

impl<W: World> Default for ObserverRegistry<W> {
    fn default() -> Self {
        Self::new()
    }
}