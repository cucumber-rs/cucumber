//! Extensions for [`ScenarioId`] to create appropriate tracing spans.

use tracing::Span;

use crate::{
    event::HookType,
    runner::basic::ScenarioId,
};

// TODO: Try remove on next Rust version update.
#[expect(clippy::allow_attributes, reason = "`#[expect]` doesn't work here")]
#[allow( // intentional
    clippy::multiple_inherent_impl,
    reason = "related to `tracing` capabilities only"
)]
impl ScenarioId {
    /// Name of the [`ScenarioId`] [`Span`] field.
    pub(crate) const SPAN_FIELD_NAME: &'static str = "__cucumber_scenario_id";

    /// Creates a new [`Span`] for running a [`Scenario`] with this
    /// [`ScenarioId`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn scenario_span(self) -> Span {
        // `Level::ERROR` is used to minimize the chance of the user-provided
        // filter to skip it.
        tracing::error_span!("scenario", __cucumber_scenario_id = self.0)
    }

    /// Creates a new [`Span`] for a running [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    #[expect(clippy::unused_self, reason = "API uniformity")]
    pub(crate) fn step_span(self, is_background: bool) -> Span {
        // `Level::ERROR` is used to minimize the chance of the user-provided
        // filter to skip it.
        if is_background {
            tracing::error_span!("background step")
        } else {
            tracing::error_span!("step")
        }
    }

    /// Creates a new [`Span`] for running a [`Hook`].
    ///
    /// [`Hook`]: crate::event::Hook
    #[expect(clippy::unused_self, reason = "API uniformity")]
    pub(crate) fn hook_span(self, hook_ty: HookType) -> Span {
        // `Level::ERROR` is used to minimize the chance of the user-provided
        // filter to skip it.
        match hook_ty {
            HookType::Before => tracing::error_span!("before hook"),
            HookType::After => tracing::error_span!("after hook"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;

    #[test]
    fn test_span_field_name_constant() {
        assert_eq!(ScenarioId::SPAN_FIELD_NAME, "__cucumber_scenario_id");
    }

    #[test]
    fn test_scenario_span_creation() {
        let scenario_id = ScenarioId(42);
        let span = scenario_id.scenario_span();
        
        assert_eq!(span.metadata().name(), "scenario");
        assert_eq!(span.metadata().level(), &Level::ERROR);
    }

    #[test]
    fn test_step_span_creation_normal() {
        let scenario_id = ScenarioId(42);
        let span = scenario_id.step_span(false);
        
        assert_eq!(span.metadata().name(), "step");
        assert_eq!(span.metadata().level(), &Level::ERROR);
    }

    #[test]
    fn test_step_span_creation_background() {
        let scenario_id = ScenarioId(42);
        let span = scenario_id.step_span(true);
        
        assert_eq!(span.metadata().name(), "background step");
        assert_eq!(span.metadata().level(), &Level::ERROR);
    }

    #[test]
    fn test_hook_span_creation_before() {
        let scenario_id = ScenarioId(42);
        let span = scenario_id.hook_span(HookType::Before);
        
        assert_eq!(span.metadata().name(), "before hook");
        assert_eq!(span.metadata().level(), &Level::ERROR);
    }

    #[test]
    fn test_hook_span_creation_after() {
        let scenario_id = ScenarioId(42);
        let span = scenario_id.hook_span(HookType::After);
        
        assert_eq!(span.metadata().name(), "after hook");
        assert_eq!(span.metadata().level(), &Level::ERROR);
    }

    #[test]
    fn test_scenario_id_display_in_span() {
        let scenario_id = ScenarioId(123);
        let span = scenario_id.scenario_span();
        
        // Verify the span contains the scenario ID field
        span.in_scope(|| {
            // The span should be active and contain the field
            assert_eq!(span.metadata().name(), "scenario");
        });
    }

    #[test]
    fn test_span_levels_are_error() {
        let scenario_id = ScenarioId(1);
        
        assert_eq!(scenario_id.scenario_span().metadata().level(), &Level::ERROR);
        assert_eq!(scenario_id.step_span(false).metadata().level(), &Level::ERROR);
        assert_eq!(scenario_id.step_span(true).metadata().level(), &Level::ERROR);
        assert_eq!(scenario_id.hook_span(HookType::Before).metadata().level(), &Level::ERROR);
        assert_eq!(scenario_id.hook_span(HookType::After).metadata().level(), &Level::ERROR);
    }

    #[test]
    fn test_different_span_names() {
        let scenario_id = ScenarioId(1);
        
        let spans = vec![
            (scenario_id.scenario_span(), "scenario"),
            (scenario_id.step_span(false), "step"),
            (scenario_id.step_span(true), "background step"),
            (scenario_id.hook_span(HookType::Before), "before hook"),
            (scenario_id.hook_span(HookType::After), "after hook"),
        ];
        
        for (span, expected_name) in spans {
            assert_eq!(span.metadata().name(), expected_name);
        }
    }
}