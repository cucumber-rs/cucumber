//! [`tracing`] integration layer.
//!
//! This module provides comprehensive tracing integration for Cucumber tests,
//! allowing for detailed logging and span management during test execution.
//! 
//! The module is organized into several focused components:
//! - [`types`]: Core type definitions and aliases
//! - [`collector`]: Event collection and scenario management
//! - [`cucumber_ext`]: Extension methods for Cucumber configuration
//! - [`scenario_id_ext`]: ScenarioId extensions for span creation
//! - [`waiter`]: Span lifecycle management
//! - [`layer`]: Tracing layer implementation
//! - [`visitor`]: Field visitors for extracting scenario information
//! - [`formatter`]: Event and field formatting with scenario markers
//! - [`writer`]: CollectorWriter for sending events to the collector

pub mod types;
pub mod collector;
pub mod cucumber_ext;
pub mod scenario_id_ext;
pub mod waiter;
pub mod layer;
pub mod visitor;
pub mod formatter;
pub mod writer;

// Re-export public types for backward compatibility
pub use collector::Collector;
pub use waiter::SpanCloseWaiter;
pub use layer::RecordScenarioId;
pub use formatter::{AppendScenarioMsg, SkipScenarioIdSpan};
pub use writer::CollectorWriter;

// Re-export commonly used type aliases
pub use types::{
    Scenarios, SpanEventsCallbacks, IsReceived, Callback,
    LogMessage, LogSender, LogReceiver, SpanCloseSender, SpanCloseReceiver,
};

// Re-export visitor types for advanced usage
pub use visitor::{GetScenarioId, IsScenarioIdSpan};

// Re-export suffix constants for parsing
pub use formatter::suffix;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_types_accessible() {
        // Test that all main types are accessible
        use std::collections::HashMap;
        use futures::channel::mpsc;
        
        let _scenarios: Scenarios = HashMap::new();
        let _span_events: SpanEventsCallbacks = HashMap::new();
        
        let (log_sender, _log_receiver): (LogSender, LogReceiver) = mpsc::unbounded();
        let (span_sender, _span_receiver): (SpanCloseSender, SpanCloseReceiver) = mpsc::unbounded();
        
        let _collector = Collector::new(_log_receiver, _span_receiver);
        let _waiter = SpanCloseWaiter::new(mpsc::unbounded().0);
        let _layer = RecordScenarioId::new(span_sender);
        let _writer = CollectorWriter::new(log_sender);
    }

    #[test]
    fn test_visitor_types_accessible() {
        let _get_visitor = GetScenarioId::new();
        let _is_visitor = IsScenarioIdSpan::new();
    }

    #[test]
    fn test_formatter_types_accessible() {
        use tracing_subscriber::fmt::format::{DefaultFields, Format};
        
        let _skip_formatter = SkipScenarioIdSpan(DefaultFields::new());
        let _append_formatter = AppendScenarioMsg(Format::default());
    }

    #[test]
    fn test_suffix_constants_accessible() {
        assert_eq!(suffix::END, "__cucumber__scenario");
        assert_eq!(suffix::BEFORE_SCENARIO_ID, "__");
        assert_eq!(suffix::NO_SCENARIO_ID, "__unknown");
    }

    #[test]
    fn test_type_aliases_work() {
        use tracing::span;
        
        let _is_received: IsReceived = true;
        let (_callback_sender, _callback_receiver): (Callback, _) = futures::channel::oneshot::channel();
        let _log_msg: LogMessage = (None, String::new());
    }

    #[test]
    fn test_module_organization() {
        // Verify all modules are accessible
        let _ = types::Scenarios::new();
        let _ = visitor::GetScenarioId::new();
        let _ = visitor::IsScenarioIdSpan::new();
        
        // Test constants are accessible from their modules
        assert!(formatter::suffix::END.len() > 0);
    }

    #[test]
    fn test_integration_types_compatibility() {
        use futures::channel::mpsc;
        use crate::runner::basic::ScenarioId;
        
        // Test that types work together as expected
        let (log_sender, log_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();
        
        let collector = Collector::new(log_receiver, span_receiver);
        let waiter = collector.scenario_span_event_waiter();
        let writer = CollectorWriter::new(log_sender);
        let layer = RecordScenarioId::new(span_sender);
        
        // These should all be compatible types
        assert!(std::mem::size_of_val(&collector) > 0);
        assert!(std::mem::size_of_val(&waiter) > 0);
        assert!(std::mem::size_of_val(&writer) > 0);
        assert!(std::mem::size_of_val(&layer) > 0);
    }
}