//! Field visitors for extracting scenario IDs and checking span properties.

use derive_more::with_trait::Debug;
use tracing::field::{Field, Visit};

use crate::runner::basic::ScenarioId;

/// [`Visit`]or extracting a [`ScenarioId`] from a
/// [`ScenarioId::SPAN_FIELD_NAME`]d [`Field`], in case it's present.
#[derive(Debug)]
pub struct GetScenarioId {
    scenario_id: Option<ScenarioId>,
}

impl GetScenarioId {
    /// Creates a new [`GetScenarioId`] visitor.
    pub const fn new() -> Self {
        Self { scenario_id: None }
    }

    /// Returns the extracted scenario ID, if any.
    pub const fn get_scenario_id(&self) -> Option<ScenarioId> {
        self.scenario_id
    }
}

impl Visit for GetScenarioId {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == ScenarioId::SPAN_FIELD_NAME {
            self.scenario_id = Some(ScenarioId(value));
        }
    }

    fn record_debug(&mut self, _: &Field, _: &dyn Debug) {}
}

/// [`Visit`]or checking whether a [`Span`] has a [`Field`] with the
/// [`ScenarioId::SPAN_FIELD_NAME`].
///
/// [`Span`]: tracing::Span
#[derive(Debug)]
pub struct IsScenarioIdSpan {
    is_scenario_span: bool,
}

impl IsScenarioIdSpan {
    /// Creates a new [`IsScenarioIdSpan`] visitor.
    pub const fn new() -> Self {
        Self {
            is_scenario_span: false,
        }
    }

    /// Returns whether the span contains a scenario ID field.
    pub const fn is_scenario_span(&self) -> bool {
        self.is_scenario_span
    }
}

impl Visit for IsScenarioIdSpan {
    fn record_debug(&mut self, field: &Field, _: &dyn Debug) {
        if field.name() == ScenarioId::SPAN_FIELD_NAME {
            self.is_scenario_span = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::field::FieldSet;

    fn create_test_field(name: &'static str) -> Field {
        let fieldset = FieldSet::new(&[name], tracing::callsite::Identifier::new(()));
        fieldset.field(name).unwrap()
    }

    #[test]
    fn test_get_scenario_id_creation() {
        let visitor = GetScenarioId::new();
        assert!(visitor.get_scenario_id().is_none());
    }

    #[test]
    fn test_get_scenario_id_extracts_correct_field() {
        let mut visitor = GetScenarioId::new();
        let field = create_test_field(ScenarioId::SPAN_FIELD_NAME);
        
        visitor.record_u64(&field, 42);
        
        assert_eq!(visitor.get_scenario_id(), Some(ScenarioId(42)));
    }

    #[test]
    fn test_get_scenario_id_ignores_other_fields() {
        let mut visitor = GetScenarioId::new();
        let field = create_test_field("other_field");
        
        visitor.record_u64(&field, 99);
        
        assert!(visitor.get_scenario_id().is_none());
    }

    #[test]
    fn test_get_scenario_id_ignores_debug_fields() {
        let mut visitor = GetScenarioId::new();
        let field = create_test_field(ScenarioId::SPAN_FIELD_NAME);
        
        visitor.record_debug(&field, &"test");
        
        assert!(visitor.get_scenario_id().is_none());
    }

    #[test]
    fn test_get_scenario_id_multiple_records() {
        let mut visitor = GetScenarioId::new();
        let correct_field = create_test_field(ScenarioId::SPAN_FIELD_NAME);
        let other_field = create_test_field("other_field");
        
        // Record other field first
        visitor.record_u64(&other_field, 99);
        assert!(visitor.get_scenario_id().is_none());
        
        // Record correct field
        visitor.record_u64(&correct_field, 42);
        assert_eq!(visitor.get_scenario_id(), Some(ScenarioId(42)));
    }

    #[test]
    fn test_get_scenario_id_overwrites() {
        let mut visitor = GetScenarioId::new();
        let field = create_test_field(ScenarioId::SPAN_FIELD_NAME);
        
        visitor.record_u64(&field, 42);
        assert_eq!(visitor.get_scenario_id(), Some(ScenarioId(42)));
        
        // Second record should overwrite
        visitor.record_u64(&field, 99);
        assert_eq!(visitor.get_scenario_id(), Some(ScenarioId(99)));
    }

    #[test]
    fn test_is_scenario_id_span_creation() {
        let visitor = IsScenarioIdSpan::new();
        assert!(!visitor.is_scenario_span());
    }

    #[test]
    fn test_is_scenario_id_span_detects_correct_field() {
        let mut visitor = IsScenarioIdSpan::new();
        let field = create_test_field(ScenarioId::SPAN_FIELD_NAME);
        
        visitor.record_debug(&field, &"test");
        
        assert!(visitor.is_scenario_span());
    }

    #[test]
    fn test_is_scenario_id_span_ignores_other_fields() {
        let mut visitor = IsScenarioIdSpan::new();
        let field = create_test_field("other_field");
        
        visitor.record_debug(&field, &"test");
        
        assert!(!visitor.is_scenario_span());
    }

    #[test]
    fn test_is_scenario_id_span_multiple_fields() {
        let mut visitor = IsScenarioIdSpan::new();
        let other_field = create_test_field("other_field");
        let scenario_field = create_test_field(ScenarioId::SPAN_FIELD_NAME);
        
        // Record other field first
        visitor.record_debug(&other_field, &"test");
        assert!(!visitor.is_scenario_span());
        
        // Record scenario field
        visitor.record_debug(&scenario_field, &"test");
        assert!(visitor.is_scenario_span());
    }

    #[test]
    fn test_is_scenario_id_span_once_true_stays_true() {
        let mut visitor = IsScenarioIdSpan::new();
        let scenario_field = create_test_field(ScenarioId::SPAN_FIELD_NAME);
        let other_field = create_test_field("other_field");
        
        visitor.record_debug(&scenario_field, &"test");
        assert!(visitor.is_scenario_span());
        
        // Recording other fields shouldn't change the result
        visitor.record_debug(&other_field, &"test");
        assert!(visitor.is_scenario_span());
    }

    #[test]
    fn test_visitor_const_methods() {
        const VISITOR1: GetScenarioId = GetScenarioId::new();
        const VISITOR2: IsScenarioIdSpan = IsScenarioIdSpan::new();
        
        assert!(VISITOR1.get_scenario_id().is_none());
        assert!(!VISITOR2.is_scenario_span());
    }

    #[test]
    fn test_field_name_consistency() {
        // Ensure the field name constant matches what we're testing
        assert_eq!(ScenarioId::SPAN_FIELD_NAME, "__cucumber_scenario_id");
    }
}