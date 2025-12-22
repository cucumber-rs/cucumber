//! State management for the summarize writer execution flow.

/// Possible states of a [`Summarize`] [`Writer`] during test execution.
///
/// This enum tracks the execution state to ensure proper timing of summary
/// output. The summary should only be generated once when the test run finishes,
/// and this state machine prevents duplicate or premature summary generation.
///
/// [`Summarize`]: super::core::Summarize
/// [`Writer`]: crate::Writer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// [`Finished`] event hasn't been encountered yet.
    ///
    /// This is the initial state during active test execution. In this state,
    /// the writer continues collecting statistics from test events.
    ///
    /// [`Finished`]: crate::event::Cucumber::Finished
    InProgress,

    /// [`Finished`] event was encountered, but summary hasn't been output yet.
    ///
    /// This is a transitional state that occurs when the test run completes
    /// but before the summary has been generated and written. This prevents
    /// the summary from being output multiple times.
    ///
    /// [`Finished`]: crate::event::Cucumber::Finished
    FinishedButNotOutput,

    /// [`Finished`] event was encountered and summary was output.
    ///
    /// This is the final state indicating that test execution is complete
    /// and the summary has been successfully generated and output. No further
    /// statistics collection or summary generation should occur.
    ///
    /// [`Finished`]: crate::event::Cucumber::Finished
    FinishedAndOutput,
}

impl State {
    /// Returns `true` if the state indicates test execution is still in progress.
    ///
    /// When in progress, the writer should continue collecting statistics from
    /// incoming test events.
    #[must_use]
    pub const fn is_in_progress(&self) -> bool {
        matches!(self, Self::InProgress)
    }

    /// Returns `true` if the state indicates testing has finished but summary
    /// output is pending.
    ///
    /// This state is used to trigger summary generation after the underlying
    /// writer has processed the final event.
    #[must_use]
    pub const fn is_finished_but_not_output(&self) -> bool {
        matches!(self, Self::FinishedButNotOutput)
    }

    /// Returns `true` if the state indicates testing has finished and summary
    /// has been output.
    ///
    /// This is the final state where no further processing should occur.
    #[must_use]
    pub const fn is_finished_and_output(&self) -> bool {
        matches!(self, Self::FinishedAndOutput)
    }

    /// Returns `true` if the state indicates testing has finished (regardless
    /// of output status).
    ///
    /// This combines both finished states to check if test execution is complete.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        matches!(self, Self::FinishedButNotOutput | Self::FinishedAndOutput)
    }

    /// Transitions the state to indicate testing has finished but summary
    /// output is pending.
    ///
    /// This should be called when a [`Finished`] event is received.
    ///
    /// [`Finished`]: crate::event::Cucumber::Finished
    ///
    /// # Returns
    ///
    /// Returns the new state after transition.
    pub fn mark_finished(&mut self) -> Self {
        if self.is_in_progress() {
            *self = Self::FinishedButNotOutput;
        }
        *self
    }

    /// Transitions the state to indicate summary has been output.
    ///
    /// This should be called after successfully outputting the summary.
    ///
    /// # Returns
    ///
    /// Returns the new state after transition.
    pub fn mark_output_complete(&mut self) -> Self {
        if self.is_finished_but_not_output() {
            *self = Self::FinishedAndOutput;
        }
        *self
    }

    /// Resets the state to in progress.
    ///
    /// This is useful for testing or when reusing a writer instance.
    ///
    /// # Returns
    ///
    /// Returns the new state after reset.
    pub fn reset(&mut self) -> Self {
        *self = Self::InProgress;
        *self
    }

    /// Returns a string representation of the state for debugging purposes.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::FinishedButNotOutput => "finished_but_not_output",
            Self::FinishedAndOutput => "finished_and_output",
        }
    }
}

impl Default for State {
    /// Returns the initial state ([`InProgress`]).
    ///
    /// [`InProgress`]: State::InProgress
    fn default() -> Self {
        Self::InProgress
    }
}

/// State manager for coordinating summary writer execution flow.
///
/// This struct provides utilities for managing state transitions and
/// ensuring proper summary generation timing.
#[derive(Clone, Copy, Debug, Default)]
pub struct StateManager {
    /// Current execution state.
    state: State,
}

impl StateManager {
    /// Creates a new state manager with initial state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::default(),
        }
    }

    /// Gets the current state.
    #[must_use]
    pub const fn current_state(&self) -> State {
        self.state
    }

    /// Checks if statistics collection should continue.
    ///
    /// Returns `true` if the writer should process events and collect statistics.
    #[must_use]
    pub fn should_collect_stats(&self) -> bool {
        self.state.is_in_progress()
    }

    /// Checks if summary output should be triggered.
    ///
    /// Returns `true` if the summary should be generated and output.
    #[must_use]
    pub fn should_output_summary(&self) -> bool {
        self.state.is_finished_but_not_output()
    }

    /// Handles the test finished event by updating state.
    ///
    /// This should be called when a [`Finished`] event is received.
    ///
    /// [`Finished`]: crate::event::Cucumber::Finished
    pub fn handle_finished_event(&mut self) {
        self.state.mark_finished();
    }

    /// Marks summary output as complete.
    ///
    /// This should be called after successfully outputting the summary.
    pub fn mark_summary_output_complete(&mut self) {
        self.state.mark_output_complete();
    }

    /// Resets the state manager to initial state.
    ///
    /// Useful for reusing the same state manager across multiple test runs.
    pub fn reset(&mut self) {
        self.state.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_initial_values() {
        let state = State::default();
        assert_eq!(state, State::InProgress);
        assert!(state.is_in_progress());
        assert!(!state.is_finished());
        assert!(!state.is_finished_but_not_output());
        assert!(!state.is_finished_and_output());
    }

    #[test]
    fn state_transition_to_finished() {
        let mut state = State::InProgress;
        let new_state = state.mark_finished();
        
        assert_eq!(state, State::FinishedButNotOutput);
        assert_eq!(new_state, State::FinishedButNotOutput);
        assert!(!state.is_in_progress());
        assert!(state.is_finished());
        assert!(state.is_finished_but_not_output());
        assert!(!state.is_finished_and_output());
    }

    #[test]
    fn state_transition_to_output_complete() {
        let mut state = State::FinishedButNotOutput;
        let new_state = state.mark_output_complete();
        
        assert_eq!(state, State::FinishedAndOutput);
        assert_eq!(new_state, State::FinishedAndOutput);
        assert!(!state.is_in_progress());
        assert!(state.is_finished());
        assert!(!state.is_finished_but_not_output());
        assert!(state.is_finished_and_output());
    }

    #[test]
    fn state_reset() {
        let mut state = State::FinishedAndOutput;
        let new_state = state.reset();
        
        assert_eq!(state, State::InProgress);
        assert_eq!(new_state, State::InProgress);
        assert!(state.is_in_progress());
    }

    #[test]
    fn state_as_str() {
        assert_eq!(State::InProgress.as_str(), "in_progress");
        assert_eq!(State::FinishedButNotOutput.as_str(), "finished_but_not_output");
        assert_eq!(State::FinishedAndOutput.as_str(), "finished_and_output");
    }

    #[test]
    fn state_invalid_transitions_ignored() {
        // mark_finished from non-InProgress states should not change state
        let mut state = State::FinishedAndOutput;
        state.mark_finished();
        assert_eq!(state, State::FinishedAndOutput);

        // mark_output_complete from non-FinishedButNotOutput states should not change state
        let mut state = State::InProgress;
        state.mark_output_complete();
        assert_eq!(state, State::InProgress);
    }

    #[test]
    fn state_manager_new() {
        let manager = StateManager::new();
        assert_eq!(manager.current_state(), State::InProgress);
        assert!(manager.should_collect_stats());
        assert!(!manager.should_output_summary());
    }

    #[test]
    fn state_manager_default() {
        let manager = StateManager::default();
        assert_eq!(manager.current_state(), State::InProgress);
    }

    #[test]
    fn state_manager_finished_event() {
        let mut manager = StateManager::new();
        
        manager.handle_finished_event();
        
        assert_eq!(manager.current_state(), State::FinishedButNotOutput);
        assert!(!manager.should_collect_stats());
        assert!(manager.should_output_summary());
    }

    #[test]
    fn state_manager_output_complete() {
        let mut manager = StateManager::new();
        manager.handle_finished_event();
        
        manager.mark_summary_output_complete();
        
        assert_eq!(manager.current_state(), State::FinishedAndOutput);
        assert!(!manager.should_collect_stats());
        assert!(!manager.should_output_summary());
    }

    #[test]
    fn state_manager_reset() {
        let mut manager = StateManager::new();
        manager.handle_finished_event();
        manager.mark_summary_output_complete();
        
        manager.reset();
        
        assert_eq!(manager.current_state(), State::InProgress);
        assert!(manager.should_collect_stats());
        assert!(!manager.should_output_summary());
    }

    #[test]
    fn state_manager_full_workflow() {
        let mut manager = StateManager::new();
        
        // Initial state
        assert!(manager.should_collect_stats());
        assert!(!manager.should_output_summary());
        
        // Test finished
        manager.handle_finished_event();
        assert!(!manager.should_collect_stats());
        assert!(manager.should_output_summary());
        
        // Summary output complete
        manager.mark_summary_output_complete();
        assert!(!manager.should_collect_stats());
        assert!(!manager.should_output_summary());
    }
}