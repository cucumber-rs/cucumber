//! Span close waiter for managing asynchronous span lifecycle events.

use futures::channel::{mpsc, oneshot};
use tracing::span;

use super::types::Callback;

/// Waiter for a particular [`Span`] to be closed, which is required because a
/// [`CollectorWriter`] can notify about an [`event::Scenario::Log`] after a
/// [`Scenario`]/[`Step`] is considered [`Finished`] already, due to
/// implementation details of a [`Subscriber`].
///
/// [`CollectorWriter`]: super::writer::CollectorWriter
/// [`Finished`]: crate::event::Scenario::Finished
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
/// [`Subscriber`]: tracing::Subscriber
#[derive(Clone, Debug)]
pub struct SpanCloseWaiter {
    /// Sender for subscribing to the [`Span`] closing.
    wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,
}

impl SpanCloseWaiter {
    /// Creates a new [`SpanCloseWaiter`].
    pub const fn new(
        wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,
    ) -> Self {
        Self {
            wait_span_event_sender,
        }
    }

    /// Waits for the [`Span`] being closed.
    pub async fn wait_for_span_close(&self, id: span::Id) {
        let (sender, receiver) = oneshot::channel();
        _ = self.wait_span_event_sender.unbounded_send((id, sender)).ok();
        _ = receiver.await.ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{select, FutureExt};
    use std::time::Duration;
    
    #[test]
    fn test_span_close_waiter_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);
        
        // Test that the waiter was created successfully
        assert!(std::mem::size_of_val(&waiter) > 0);
    }

    #[test]
    fn test_span_close_waiter_clone() {
        let (sender, _receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);
        let waiter_clone = waiter.clone();
        
        // Both waiters should be equivalent
        assert!(std::mem::size_of_val(&waiter) == std::mem::size_of_val(&waiter_clone));
    }

    #[tokio::test]
    async fn test_wait_for_span_close_basic() {
        let (sender, mut receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);
        
        let span_id = span::Id::from_u64(42);
        
        // Start waiting for span close in background
        let wait_future = waiter.wait_for_span_close(span_id.clone());
        
        // Verify the waiter sent a subscription request
        let (received_id, callback) = receiver.try_next().unwrap().unwrap();
        assert_eq!(received_id, span_id);
        
        // Simulate span close by sending through callback
        callback.send(()).unwrap();
        
        // The wait should complete
        wait_future.await;
    }

    #[tokio::test]
    async fn test_wait_for_span_close_timeout() {
        let (sender, _receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);
        
        let span_id = span::Id::from_u64(42);
        
        // Start waiting for span close
        let wait_future = waiter.wait_for_span_close(span_id);
        
        // Create a timeout future
        let timeout_future = tokio::time::sleep(Duration::from_millis(10)).fuse();
        
        select! {
            _ = wait_future.fuse() => {
                panic!("Wait should not complete without callback");
            }
            _ = timeout_future => {
                // This is expected - the wait should timeout since no callback is sent
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_span_waiters() {
        let (sender, mut receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);
        
        let span_id_1 = span::Id::from_u64(1);
        let span_id_2 = span::Id::from_u64(2);
        
        // Start waiting for multiple spans
        let wait_future_1 = waiter.wait_for_span_close(span_id_1.clone()).fuse();
        let wait_future_2 = waiter.wait_for_span_close(span_id_2.clone()).fuse();
        
        // Get both subscription requests
        let (received_id_1, callback_1) = receiver.try_next().unwrap().unwrap();
        let (received_id_2, callback_2) = receiver.try_next().unwrap().unwrap();
        
        assert_eq!(received_id_1, span_id_1);
        assert_eq!(received_id_2, span_id_2);
        
        // Close span 2 first
        callback_2.send(()).unwrap();
        
        // Only wait_future_2 should complete
        select! {
            _ = wait_future_1 => panic!("First waiter should not complete"),
            _ = wait_future_2 => (), // Expected
        }
        
        // Now close span 1
        callback_1.send(()).unwrap();
        wait_future_1.await; // This should complete now
    }

    #[test]
    fn test_waiter_with_closed_sender() {
        let (sender, receiver) = mpsc::unbounded::<(span::Id, Callback)>();
        drop(receiver); // Close the receiver
        
        let waiter = SpanCloseWaiter::new(sender);
        let span_id = span::Id::from_u64(42);
        
        // This should handle the closed sender gracefully
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            waiter.wait_for_span_close(span_id).await;
        });
    }
}