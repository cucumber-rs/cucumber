//! Tracing layer for recording scenario IDs and managing span lifecycle.

use futures::channel::mpsc;
use tracing::{Subscriber, span};
use tracing_subscriber::{
    layer::{self, Layer},
    registry::LookupSpan,
};

use crate::runner::basic::ScenarioId;
use super::visitor::GetScenarioId;

/// [`Layer`] recording a [`ScenarioId`] into [`Span`]'s [`Extensions`].
///
/// [`Extensions`]: tracing_subscriber::registry::Extensions
#[derive(Debug)]
pub struct RecordScenarioId {
    /// Sender for [`Span`] closing events.
    span_close_sender: mpsc::UnboundedSender<span::Id>,
}

impl RecordScenarioId {
    /// Creates a new [`RecordScenarioId`] [`Layer`].
    pub const fn new(span_close_sender: mpsc::UnboundedSender<span::Id>) -> Self {
        Self { span_close_sender }
    }
}

impl<S> Layer<S> for RecordScenarioId
where
    S: for<'a> LookupSpan<'a> + Subscriber,
{
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = GetScenarioId::new();
            attrs.values().record(&mut visitor);

            if let Some(scenario_id) = visitor.get_scenario_id() {
                let mut ext = span.extensions_mut();
                _ = ext.replace(scenario_id);
            }
        }
    }

    fn on_record(
        &self,
        id: &span::Id,
        values: &span::Record<'_>,
        ctx: layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = GetScenarioId::new();
            values.record(&mut visitor);

            if let Some(scenario_id) = visitor.get_scenario_id() {
                let mut ext = span.extensions_mut();
                _ = ext.replace(scenario_id);
            }
        }
    }

    fn on_close(&self, id: span::Id, _ctx: layer::Context<'_, S>) {
        _ = self.span_close_sender.unbounded_send(id).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{Event, Subscriber};
    use tracing_subscriber::{layer::Context, registry::Registry};
    use std::sync::{Arc, Mutex};
    use std::collections::HashSet;

    #[derive(Debug)]
    struct TestSubscriber {
        spans: Arc<Mutex<HashSet<span::Id>>>,
    }

    impl TestSubscriber {
        fn new() -> Self {
            Self {
                spans: Arc::new(Mutex::new(HashSet::new())),
            }
        }
    }

    impl Subscriber for TestSubscriber {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
            let id = span::Id::from_u64(42);
            self.spans.lock().unwrap().insert(id.clone());
            id
        }

        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn event(&self, _event: &Event<'_>) {}

        fn enter(&self, _span: &span::Id) {}

        fn exit(&self, _span: &span::Id) {}
    }

    #[test]
    fn test_record_scenario_id_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let layer = RecordScenarioId::new(sender);
        
        // Test that the layer was created successfully
        assert!(std::mem::size_of_val(&layer) > 0);
    }

    #[test]
    fn test_layer_sends_span_close_events() {
        let (sender, mut receiver) = mpsc::unbounded();
        let layer = RecordScenarioId::new(sender);
        
        let span_id = span::Id::from_u64(42);
        let subscriber = Registry::default();
        let ctx = Context::new(&subscriber);
        
        // Simulate span close
        layer.on_close(span_id.clone(), ctx);
        
        // Verify the span close event was sent
        let received_id = receiver.try_next().unwrap().unwrap();
        assert_eq!(received_id, span_id);
    }

    #[test]
    fn test_multiple_span_close_events() {
        let (sender, mut receiver) = mpsc::unbounded();
        let layer = RecordScenarioId::new(sender);
        
        let subscriber = Registry::default();
        let ctx = Context::new(&subscriber);
        
        let span_ids = vec![
            span::Id::from_u64(1),
            span::Id::from_u64(2),
            span::Id::from_u64(3),
        ];
        
        // Send multiple span close events
        for span_id in &span_ids {
            layer.on_close(span_id.clone(), ctx.clone());
        }
        
        // Verify all events were received
        for expected_id in span_ids {
            let received_id = receiver.try_next().unwrap().unwrap();
            assert_eq!(received_id, expected_id);
        }
    }

    #[test]
    fn test_on_new_span_with_no_scenario_id() {
        let (sender, _receiver) = mpsc::unbounded();
        let layer = RecordScenarioId::new(sender);
        
        let subscriber = Registry::default();
        let span_id = span::Id::from_u64(1);
        
        // Create span attributes without scenario ID
        let metadata = tracing::Metadata::new(
            "test_span",
            "test_target",
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier::new(())),
            tracing::metadata::Kind::SPAN,
        );
        
        let values = metadata.fields().value_set(&[]);
        let attrs = span::Attributes::new(&metadata, &values);
        let ctx = Context::new(&subscriber);
        
        // This should not panic even with no scenario ID
        layer.on_new_span(&attrs, &span_id, ctx);
    }

    #[test]
    fn test_on_record_with_no_scenario_id() {
        let (sender, _receiver) = mpsc::unbounded();
        let layer = RecordScenarioId::new(sender);
        
        let subscriber = Registry::default();
        let span_id = span::Id::from_u64(1);
        
        let metadata = tracing::Metadata::new(
            "test_span",
            "test_target",
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier::new(())),
            tracing::metadata::Kind::SPAN,
        );
        
        let values = metadata.fields().value_set(&[]);
        let record = span::Record::new(&values);
        let ctx = Context::new(&subscriber);
        
        // This should not panic even with no scenario ID
        layer.on_record(&span_id, &record, ctx);
    }

    #[test]
    fn test_layer_with_closed_sender() {
        let (sender, receiver) = mpsc::unbounded();
        drop(receiver); // Close receiver
        
        let layer = RecordScenarioId::new(sender);
        let subscriber = Registry::default();
        let ctx = Context::new(&subscriber);
        let span_id = span::Id::from_u64(42);
        
        // This should handle the closed sender gracefully
        layer.on_close(span_id, ctx);
    }
}