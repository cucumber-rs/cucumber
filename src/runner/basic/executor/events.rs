//! Event sending logic for the Basic executor.

use futures::channel::mpsc;

use crate::{Event, World, event, parser};

/// Event sending functionality for the Executor.
pub(super) struct EventSender<W> {
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
}

impl<W: World> EventSender<W> {
    /// Creates a new EventSender.
    pub(super) fn new(sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>) -> Self {
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
}