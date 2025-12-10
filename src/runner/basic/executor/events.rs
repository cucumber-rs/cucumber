//! Event sending logic for the Basic executor.

use futures::channel::mpsc;

use crate::{Event, World, event, parser};

/// Event sending functionality for the Executor.
pub(super) struct EventSender<W> {
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
}

impl<W: World> EventSender<W> {
    /// Creates a new EventSender.
    pub(super) fn new_with_sender(sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>) -> Self {
        Self { sender }
    }

    /// Sends a single event.
    pub(super) fn send_event(&self, event: event::Cucumber<W>) {
        self.sender
            .unbounded_send(Ok(Event::new(event)))
            .unwrap_or_else(|e| panic!("Failed to send `Cucumber` event: {e}"));
    }

    /// Sends multiple events.
    pub(super) fn send_all_events(
        &self,
        events: impl IntoIterator<Item = event::Cucumber<W>>,
    ) {
        for event in events {
            self.send_event(event);
        }
    }

    /// Sends an event with additional metadata.
    pub(super) fn send_event_with_meta(
        &self,
        event: event::Cucumber<W>,
        _meta: &crate::event::Metadata,
    ) {
        // Currently just sends the event, but could be extended to include metadata
        self.send_event(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event, test_utils::common::TestWorld};
    use futures::{channel::mpsc, TryStreamExt as _};

    #[test]
    fn test_event_sender_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);
        
        // EventSender should be created successfully
        assert!(true); // Basic existence check
    }

    #[test]
    fn test_send_single_event() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);
        
        let event = event::Cucumber::<TestWorld>::Started;
        event_sender.send_event(event);
        
        // Should receive the event
        let received = receiver.try_next().unwrap().unwrap().unwrap();
        assert!(matches!(received.value, event::Cucumber::Started));
    }

    #[test]
    fn test_send_multiple_events() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);
        
        let events = vec![
            event::Cucumber::<TestWorld>::Started,
            event::Cucumber::<TestWorld>::Finished,
        ];
        
        event_sender.send_all_events(events);
        
        // Should receive both events
        let first = receiver.try_next().unwrap().unwrap().unwrap();
        let second = receiver.try_next().unwrap().unwrap().unwrap();
        
        assert!(matches!(first.value, event::Cucumber::Started));
        assert!(matches!(second.value, event::Cucumber::Finished));
    }

    #[test]
    fn test_send_event_with_meta() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);
        
        let event = event::Cucumber::<TestWorld>::Started;
        let meta = crate::event::Metadata::new(());
        
        event_sender.send_event_with_meta(event, &meta);
        
        // Should receive the event
        let received = receiver.try_next().unwrap().unwrap().unwrap();
        assert!(matches!(received.value, event::Cucumber::Started));
    }

    #[test]
    #[should_panic(expected = "Failed to send `Cucumber` event")]
    fn test_send_event_panics_on_closed_channel() {
        let (sender, receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);
        
        // Close the receiver to make the channel closed
        drop(receiver);
        
        let event = event::Cucumber::<TestWorld>::Started;
        event_sender.send_event(event); // Should panic
    }

    #[test]
    fn test_event_sender_multiple_instances() {
        let (sender1, mut receiver1) = mpsc::unbounded();
        let (sender2, mut receiver2) = mpsc::unbounded();
        
        let event_sender1 = EventSender::<TestWorld>::new_with_sender(sender1);
        let event_sender2 = EventSender::<TestWorld>::new_with_sender(sender2);
        
        event_sender1.send_event(event::Cucumber::<TestWorld>::Started);
        event_sender2.send_event(event::Cucumber::<TestWorld>::Finished);
        
        let received1 = receiver1.try_next().unwrap().unwrap().unwrap();
        let received2 = receiver2.try_next().unwrap().unwrap().unwrap();
        
        assert!(matches!(received1.value, event::Cucumber::Started));
        assert!(matches!(received2.value, event::Cucumber::Finished));
    }
}