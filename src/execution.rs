//! Explicit completion requests bound to one runner-owned invocation.
//!
//! Requests do not return from user code. Return after completing a request;
//! a later panic or timeout remains a failure. Retained handles never retarget.
use std::{
    fmt,
    sync::{Arc, Mutex},
};

/// Deliberate non-passing completion, distinct from an undefined definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Completion {
    /// Work is not implemented yet; strictness controls final run failure.
    Pending,
    /// Work was explicitly skipped; this alone does not fail the run.
    Skipped,
}
/// Rejection of a request outside its enabled original invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ControlError {
    /// The runner did not enable explicit completion control.
    Disabled,
    /// No body or callback is currently executing.
    NoActiveScope,
    /// The handle belongs to a completed or cancelled invocation.
    ClosedScope,
    /// This invocation already has a completion request.
    AlreadyCompleted,
}
impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disabled => "execution control is disabled",
            Self::NoActiveScope => "no active execution scope",
            Self::ClosedScope => "execution scope is closed",
            Self::AlreadyCompleted => "completion already requested",
        })
    }
}
impl std::error::Error for ControlError {}
#[derive(Debug)]
struct Scope {
    open: bool,
    completion: Option<Completion>,
}
#[derive(Debug, Default)]
struct State {
    active: Mutex<Option<Arc<Mutex<Scope>>>>,
}
/// Access to the current invocation; acquire a bound handle inside each callback.
#[derive(Clone, Debug, Default)]
pub struct ExecutionControl(Option<Arc<State>>);
impl ExecutionControl {
    pub(crate) fn enabled() -> Self {
        Self(Some(Arc::new(State::default())))
    }
    /// Acquires an immutable binding to the current callback/body invocation.
    pub fn current(&self) -> Result<ExecutionHandle, ControlError> {
        let state = self.0.as_ref().ok_or(ControlError::Disabled)?;
        let active = state.active.lock().expect("execution control poisoned");
        let scope = active.as_ref().ok_or(ControlError::NoActiveScope)?.clone();
        Ok(ExecutionHandle(scope))
    }
    pub(crate) fn enter(&self) -> InvocationGuard {
        let Some(state) = &self.0 else {
            return InvocationGuard { state: None, scope: None };
        };
        let scope =
            Arc::new(Mutex::new(Scope { open: true, completion: None }));
        let mut active =
            state.active.lock().expect("execution control poisoned");
        assert!(active.is_none(), "overlapping runner execution scopes");
        *active = Some(scope.clone());
        drop(active);
        InvocationGuard { state: Some(state.clone()), scope: Some(scope) }
    }
}
/// A request handle bound to its original callback, step or attempt.
#[derive(Clone, Debug)]
pub struct ExecutionHandle(Arc<Mutex<Scope>>);
impl ExecutionHandle {
    /// Records the first requested completion. This does not stop user code.
    pub fn complete(&self, completion: Completion) -> Result<(), ControlError> {
        let mut scope = self.0.lock().expect("execution control poisoned");
        if !scope.open {
            return Err(ControlError::ClosedScope);
        }
        if scope.completion.is_some() {
            return Err(ControlError::AlreadyCompleted);
        }
        scope.completion = Some(completion);
        Ok(())
    }
}
pub(crate) struct InvocationGuard {
    state: Option<Arc<State>>,
    scope: Option<Arc<Mutex<Scope>>>,
}
impl InvocationGuard {
    pub(crate) fn finish(mut self) -> Option<Completion> {
        self.close()
    }
    fn close(&mut self) -> Option<Completion> {
        let scope = self.scope.take()?;
        let result = {
            let mut current = scope.lock().expect("execution control poisoned");
            current.open = false;
            current.completion
        };
        if let Some(state) = &self.state {
            let mut active =
                state.active.lock().expect("execution control poisoned");
            if active.as_ref().is_some_and(|value| Arc::ptr_eq(value, &scope)) {
                *active = None;
            }
        }
        result
    }
}
impl Drop for InvocationGuard {
    fn drop(&mut self) {
        self.close();
    }
}
/// Runner-provided contexts for an additive World constructor.
#[derive(Clone, Debug, Default)]
pub struct WorldContext {
    attachments: crate::attachment::AttachmentContext,
    execution: ExecutionControl,
}
impl WorldContext {
    pub(crate) fn new(
        attachments: crate::attachment::AttachmentContext,
        execution: ExecutionControl,
    ) -> Self {
        Self { attachments, execution }
    }
    /// Existing typed evidence context, preserving attachment-aware constructors.
    #[must_use]
    pub fn attachments(&self) -> crate::attachment::AttachmentContext {
        self.attachments.clone()
    }
    /// Current-invocation completion access, disabled unless explicitly selected.
    #[must_use]
    pub fn execution(&self) -> ExecutionControl {
        self.execution.clone()
    }
}

/// Actual completed status, ordered by the pinned Cucumber result severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[non_exhaustive]
pub enum OutcomeStatus {
    /// The invocation passed.
    Passed,
    /// The invocation or selected slot was skipped.
    Skipped,
    /// The invocation explicitly marked its work pending.
    Pending,
    /// No definition matched the planned step.
    Undefined,
    /// More than one definition matched the planned step.
    Ambiguous,
    /// Execution failed, including panic or timeout.
    Failed,
}
impl From<Completion> for OutcomeStatus {
    fn from(completion: Completion) -> Self {
        match completion {
            Completion::Pending => Self::Pending,
            Completion::Skipped => Self::Skipped,
        }
    }
}
/// Authoritative status and policy flags carried by an opt-in native event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Outcome {
    /// Actual completion status; native event variants are only a projection.
    pub status: OutcomeStatus,
    /// Whether this status makes the final attempt unsuccessful under its policy.
    pub fails_run: bool,
    /// Whether the status is retry-eligible; a retry also requires attempts left.
    pub retryable: bool,
}
/// Explicit outcome behavior; existing native semantics remain the default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum OutcomePolicy {
    /// Existing execution/event semantics with completion control disabled.
    #[default]
    Legacy,
    /// Pinned Cucumber.js outcome, skipping and retry semantics.
    CucumberJs {
        /// Whether pending work makes the run unsuccessful.
        strict_pending: bool,
    },
}
impl OutcomePolicy {
    /// Selects Cucumber.js semantics with its default strict pending behavior.
    #[must_use]
    pub const fn cucumber_js() -> Self {
        Self::CucumberJs { strict_pending: true }
    }
    /// Computes authoritative flags without consuming retry availability.
    #[must_use]
    pub const fn outcome(self, status: OutcomeStatus) -> Outcome {
        use OutcomeStatus::{Ambiguous, Failed, Pending, Undefined};
        let (fails_run, retryable) = match self {
            Self::Legacy => (
                matches!(status, Failed | Ambiguous),
                matches!(status, Failed | Ambiguous),
            ),
            Self::CucumberJs { strict_pending } => (
                matches!(status, Failed | Ambiguous | Undefined)
                    || (strict_pending && matches!(status, Pending)),
                matches!(status, Failed),
            ),
        };
        Outcome { status, fails_run, retryable }
    }
}

/// Whether selected scenarios execute application code or inspect definitions.
///
/// Dry-run still constructs each selected World, but never invokes lifecycle
/// callbacks or step bodies. OutcomePolicy is retained independently for Run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ExecutionMode {
    /// Execute normally using the selected outcome and timeout policies.
    #[default]
    Run,
    /// Construct Worlds and report skipped, undefined or ambiguous definitions.
    DryRun,
}
impl ExecutionMode {
    pub(crate) fn outcome(
        self,
        policy: OutcomePolicy,
        status: OutcomeStatus,
    ) -> Outcome {
        match self {
            Self::Run => policy.outcome(status),
            Self::DryRun => Outcome {
                status,
                fails_run: status == OutcomeStatus::Failed,
                retryable: false,
            },
        }
    }
}
