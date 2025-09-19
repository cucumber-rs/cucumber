//! Step execution context and related types.
//!
//! This module provides the [`Context`] struct that contains information
//! about the step being executed, including the step itself and any regex
//! capture groups from the step matching process.

/// Name of a capturing group inside a [`regex`].
pub type CaptureName = Option<String>;

/// Context for a [`Step`] function execution.
#[derive(Clone, Debug)]
pub struct Context {
    /// [`Step`] matched to a [`Step`] function.
    ///
    /// [`Step`]: gherkin::Step
    pub step: gherkin::Step,

    /// [`Regex`] matches of a [`Step::value`].
    ///
    /// [`Step::value`]: gherkin::Step::value
    pub matches: Vec<(CaptureName, String)>,
}

impl Context {
    /// Creates a new [`Context`] with the given step and matches.
    #[must_use]
    pub fn new(step: gherkin::Step, matches: Vec<(CaptureName, String)>) -> Self {
        Self { step, matches }
    }

    /// Returns a reference to the step.
    #[must_use]
    pub fn step(&self) -> &gherkin::Step {
        &self.step
    }

    /// Returns a reference to the regex matches.
    #[must_use]
    pub fn matches(&self) -> &[(CaptureName, String)] {
        &self.matches
    }

    /// Returns the value of a named capture group, if it exists.
    #[must_use]
    pub fn get_named_capture(&self, name: &str) -> Option<&str> {
        self.matches
            .iter()
            .find(|(capture_name, _)| {
                capture_name.as_ref().map_or(false, |n| n == name)
            })
            .map(|(_, value)| value.as_str())
    }

    /// Returns the value of a capture group by index (0 is the whole match).
    #[must_use]
    pub fn get_capture(&self, index: usize) -> Option<&str> {
        self.matches.get(index).map(|(_, value)| value.as_str())
    }

    /// Returns the number of capture groups (including the whole match).
    #[must_use]
    pub fn capture_count(&self) -> usize {
        self.matches.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gherkin::{Step as GherkinStep, StepType};

    fn create_test_step() -> GherkinStep {
        GherkinStep {
            keyword: "Given".to_string(),
            ty: StepType::Given,
            value: "I have 5 cucumbers".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
        }
    }

    #[test]
    fn context_new_creates_context_with_step_and_matches() {
        let step = create_test_step();
        let matches = vec![
            (None, "I have 5 cucumbers".to_string()),
            (Some("count".to_string()), "5".to_string()),
        ];
        
        let context = Context::new(step.clone(), matches.clone());
        assert_eq!(context.step.value, step.value);
        assert_eq!(context.matches, matches);
    }

    #[test]
    fn context_step_returns_step_reference() {
        let step = create_test_step();
        let context = Context::new(step.clone(), vec![]);
        
        assert_eq!(context.step().value, step.value);
        assert_eq!(context.step().ty, step.ty);
    }

    #[test]
    fn context_matches_returns_matches_reference() {
        let step = create_test_step();
        let matches = vec![
            (None, "whole match".to_string()),
            (Some("group1".to_string()), "value1".to_string()),
        ];
        
        let context = Context::new(step, matches.clone());
        assert_eq!(context.matches(), &matches);
    }

    #[test]
    fn context_get_named_capture_returns_correct_value() {
        let step = create_test_step();
        let matches = vec![
            (None, "I have 5 cucumbers".to_string()),
            (Some("count".to_string()), "5".to_string()),
            (Some("item".to_string()), "cucumbers".to_string()),
        ];
        
        let context = Context::new(step, matches);
        assert_eq!(context.get_named_capture("count"), Some("5"));
        assert_eq!(context.get_named_capture("item"), Some("cucumbers"));
        assert_eq!(context.get_named_capture("nonexistent"), None);
    }

    #[test]
    fn context_get_capture_returns_correct_value_by_index() {
        let step = create_test_step();
        let matches = vec![
            (None, "whole match".to_string()),
            (Some("group1".to_string()), "value1".to_string()),
            (Some("group2".to_string()), "value2".to_string()),
        ];
        
        let context = Context::new(step, matches);
        assert_eq!(context.get_capture(0), Some("whole match"));
        assert_eq!(context.get_capture(1), Some("value1"));
        assert_eq!(context.get_capture(2), Some("value2"));
        assert_eq!(context.get_capture(3), None);
    }

    #[test]
    fn context_capture_count_returns_correct_count() {
        let step = create_test_step();
        let matches = vec![
            (None, "whole match".to_string()),
            (Some("group1".to_string()), "value1".to_string()),
        ];
        
        let context = Context::new(step, matches);
        assert_eq!(context.capture_count(), 2);
    }

    #[test]
    fn context_clone_works() {
        let step = create_test_step();
        let matches = vec![
            (None, "test".to_string()),
            (Some("group".to_string()), "value".to_string()),
        ];
        
        let context = Context::new(step.clone(), matches.clone());
        let cloned = context.clone();
        
        assert_eq!(cloned.step.value, step.value);
        assert_eq!(cloned.matches, matches);
    }

    #[test]
    fn context_debug_format_works() {
        let step = create_test_step();
        let matches = vec![
            (None, "test".to_string()),
        ];
        
        let context = Context::new(step, matches);
        let debug_output = format!("{:?}", context);
        assert!(debug_output.contains("Context"));
        assert!(debug_output.contains("step"));
        assert!(debug_output.contains("matches"));
    }
}