//! Type definitions and aliases for the tracing integration.

use std::collections::HashMap;
use futures::channel::{mpsc, oneshot};
use tracing::span;

use crate::{
    event::Source,
    runner::basic::{RetryOptions, ScenarioId},
};

/// [`HashMap`] from a [`ScenarioId`] to its [`Scenario`] and full path.
///
/// [`Scenario`]: gherkin::Scenario
pub type Scenarios = HashMap<
    ScenarioId,
    (
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
        Option<RetryOptions>,
    ),
>;

/// All [`Callback`]s for [`Span`]s closing events with their completion status.
pub type SpanEventsCallbacks = HashMap<span::Id, (Option<Vec<Callback>>, IsReceived)>;

/// Indication whether a [`Span`] closing event was received.
pub type IsReceived = bool;

/// Callback for notifying a [`Runner`] about a [`Span`] being closed.
pub type Callback = oneshot::Sender<()>;

/// Message structure for tracing events with optional scenario ID.
pub type LogMessage = (Option<ScenarioId>, String);

/// Sender for log messages.
pub type LogSender = mpsc::UnboundedSender<LogMessage>;

/// Receiver for log messages.
pub type LogReceiver = mpsc::UnboundedReceiver<LogMessage>;

/// Sender for span close events.
pub type SpanCloseSender = mpsc::UnboundedSender<span::Id>;

/// Receiver for span close events.
pub type SpanCloseReceiver = mpsc::UnboundedReceiver<span::Id>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_scenarios_type_alias() {
        let scenarios: Scenarios = HashMap::new();
        assert_eq!(scenarios.len(), 0);
    }

    #[test]
    fn test_span_events_callbacks_type() {
        let callbacks: SpanEventsCallbacks = HashMap::new();
        assert_eq!(callbacks.len(), 0);
    }

    #[test]
    fn test_is_received_type() {
        let received: IsReceived = true;
        assert!(received);
        let not_received: IsReceived = false;
        assert!(!not_received);
    }

    #[test]
    fn test_log_message_type() {
        let (log_sender, _log_receiver): (LogSender, LogReceiver) = mpsc::unbounded();
        let message: LogMessage = (None, "test message".to_string());
        assert!(log_sender.unbounded_send(message).is_ok());
    }

    #[test]
    fn test_span_close_channels() {
        let (span_sender, _span_receiver): (SpanCloseSender, SpanCloseReceiver) = mpsc::unbounded();
        let span_id = span::Id::from_u64(42);
        assert!(span_sender.unbounded_send(span_id).is_ok());
    }

    #[test]
    fn test_callback_type() {
        let (_callback_sender, callback_receiver): (Callback, oneshot::Receiver<()>) = oneshot::channel();
        // Callback should be a oneshot sender
        drop(callback_receiver);
    }

    #[test]
    fn test_span_events_with_callbacks() {
        let mut callbacks: SpanEventsCallbacks = HashMap::new();
        let span_id = span::Id::from_u64(1);
        let (sender, _receiver) = oneshot::channel();
        
        callbacks.insert(span_id, (Some(vec![sender]), false));
        assert_eq!(callbacks.len(), 1);
    }
}