//! Actual scenario-hook selections, invocations and native cleanup context.
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use super::basic::ScenarioId;
use crate::{
    event::{ScenarioFinished, Source, StepError},
    step,
};

/// Stage assigned at registration, independent of empty-scenario compatibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioHookKind {
    /// Setup in registration order.
    Before,
    /// Cleanup in reverse registration order.
    After,
}
/// Handling of registered cleanup when the scenario execution plan has no
/// steps.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EmptyScenarioHooks {
    /// Preserve native cleanup and its body snapshot, including after setup
    /// failure.
    #[default]
    Native,
    /// Follow Cucumber.js 13.3's empty-Pickle setup classification for cleanup.
    CucumberJs13_3,
}
/// Stable registration metadata; source identifies the real Rust registration
/// call.
#[derive(Clone, Debug)]
pub struct ScenarioHookDefinition {
    /// Process-local registration identity, also identifying its selected slot.
    pub id: u64,
    /// Caller-supplied name.
    pub name: String,
    /// Registered lifecycle stage.
    pub kind: ScenarioHookKind,
    /// Actual registration location, rather than a guessed callback definition.
    pub location: step::Location,
}
/// Actual native scenario attempt containing an invocation or selection
/// decision.
#[derive(Clone, Debug)]
pub struct ScenarioHookAttempt {
    /// Native feature, including its input path.
    pub feature: Source<gherkin::Feature>,
    /// Native Rule, if present.
    pub rule: Option<Source<gherkin::Rule>>,
    /// Expanded native scenario.
    pub scenario: Source<gherkin::Scenario>,
    /// Runner identity; retries receive distinct identities.
    pub scenario_id: ScenarioId,
    /// Zero-based retry attempt.
    pub attempt: usize,
    /// Remaining retries configured by native runner policy.
    pub retries_remaining: usize,
    /// Whether the actual native plan contains background or scenario steps.
    pub has_steps: bool,
}
/// Explicit selected slots, including empty selection and legacy stage
/// replacement.
#[derive(Clone, Debug)]
pub struct ScenarioHookPlan {
    /// Actual attempt owning this selection.
    pub attempt: ScenarioHookAttempt,
    /// Whether a registry setup stage exists, even if its filters select
    /// nothing.
    pub before_stage: bool,
    /// Whether a registry cleanup stage exists, even if its filters select
    /// nothing.
    pub after_stage: bool,
    /// Selected setup slots in execution order.
    pub before: Vec<ScenarioHookDefinition>,
    /// Selected cleanup slots in execution order.
    pub after: Vec<ScenarioHookDefinition>,
}
/// A failed callback retained separately from a native body outcome.
#[derive(Clone, Debug)]
pub struct ScenarioHookFailure {
    /// Actual failing registration.
    pub definition: ScenarioHookDefinition,
    /// Captured panic diagnostic.
    pub message: String,
}
/// Honest classification of the native result, without an invented JS error
/// object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioHookStatus {
    /// The invocation explicitly skipped its work.
    Skipped,
    /// The invocation explicitly marked its work pending.
    Pending,
    /// No body or callback failure has occurred.
    Passed,
    /// The native step had no matching definition.
    Undefined,
    /// The native step matched more than one definition.
    Ambiguous,
    /// A native body, setup or cleanup operation failed.
    Failed,
}
/// Native state observed immediately before one scoped cleanup invocation.
///
/// This is not a complete JavaScript TestStepResult: it deliberately has no
/// guessed raw error or duration, and retries follow native runner policy.
#[derive(Clone, Debug)]
pub struct ScenarioHookResult {
    /// Current native outcome, including preceding callback failures.
    pub status: ScenarioHookStatus,
    /// First diagnostic of the currently most severe native outcome.
    pub primary_failure: Option<String>,
    /// Each earlier callback failure, retained independently.
    pub previous_hook_failures: Vec<ScenarioHookFailure>,
    /// Whether a retry is available and native policy would retry this outcome
    /// now.
    pub retry_if_finished_now: bool,
}
impl ScenarioHookResult {
    pub(crate) fn from_body(
        body: &ScenarioFinished,
        failures: Vec<ScenarioHookFailure>,
        remaining: usize,
    ) -> Self {
        let (status, primary_failure) = match body {
            ScenarioFinished::StepPassed => (ScenarioHookStatus::Passed, None),
            ScenarioFinished::StepSkipped => {
                (ScenarioHookStatus::Undefined, None)
            }
            ScenarioFinished::BeforeHookFailed(info) => (
                ScenarioHookStatus::Failed,
                Some(super::step_hooks::panic_message(info.as_ref())),
            ),
            ScenarioFinished::StepFailed(
                _,
                _,
                StepError::AmbiguousMatch(error),
            ) => (ScenarioHookStatus::Ambiguous, Some(error.to_string())),
            ScenarioFinished::StepFailed(_, _, StepError::NotFound) => {
                (ScenarioHookStatus::Undefined, None)
            }
            ScenarioFinished::StepFailed(_, _, error) => {
                (ScenarioHookStatus::Failed, Some(error.to_string()))
            }
        };
        let retry_if_finished_now = remaining > 0
            && matches!(
                status,
                ScenarioHookStatus::Failed | ScenarioHookStatus::Ambiguous
            );
        Self {
            status,
            primary_failure,
            previous_hook_failures: failures,
            retry_if_finished_now,
        }
    }
    pub(crate) fn apply_outcome(
        &mut self,
        outcome: crate::execution::Outcome,
        remaining: usize,
    ) {
        use crate::execution::OutcomeStatus as S;
        self.status = match outcome.status {
            S::Passed => ScenarioHookStatus::Passed,
            S::Skipped => ScenarioHookStatus::Skipped,
            S::Pending => ScenarioHookStatus::Pending,
            S::Undefined => ScenarioHookStatus::Undefined,
            S::Ambiguous => ScenarioHookStatus::Ambiguous,
            S::Failed => ScenarioHookStatus::Failed,
        };
        self.retry_if_finished_now = remaining > 0 && outcome.retryable;
    }
    pub(crate) fn failed(
        &mut self,
        failure: ScenarioHookFailure,
        remaining: usize,
    ) {
        if self.status != ScenarioHookStatus::Failed {
            self.primary_failure = Some(failure.message.clone());
        }
        self.status = ScenarioHookStatus::Failed;
        self.retry_if_finished_now = remaining > 0;
        self.previous_hook_failures.push(failure);
    }
}
/// Rich context supplied only to the additive scoped callback APIs.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ScenarioHookScope {
    /// Actual native attempt.
    pub attempt: ScenarioHookAttempt,
    /// Actual registration currently invoked.
    pub definition: ScenarioHookDefinition,
    /// Unique actual invocation identity; skipped slots never receive one.
    pub invocation_id: u64,
    /// Original pre-cleanup native result, absent during setup.
    pub body_snapshot: Option<ScenarioFinished>,
    /// Evolving prior result; absent during setup and JS13.3 empty-Pickle
    /// cleanup.
    pub result: Option<ScenarioHookResult>,
    /// Authoritative prior result under an explicitly selected outcome policy.
    pub outcome: Option<crate::execution::Outcome>,
}
/// Why a selected callback slot was not invoked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioHookSkipReason {
    /// Explicit inspection selected the slot without invoking application code.
    DryRun,
    /// A prior pending or skipped callback prevented this setup-classified
    /// slot.
    PreviousNonPassing,
    /// An earlier selected callback prevented this setup-classified invocation.
    PreviousFailure,
    /// Native World initialization failed before callback execution.
    WorldInitializationFailed,
}
/// Actual execution or non-execution of one selected callback slot.
#[derive(Clone, Debug)]
pub enum ScenarioHookDisposition {
    /// A callback was constructed and polled, with captured timing and result.
    Executed {
        /// Unique invocation identity.
        invocation_id: u64,
        /// Observed time immediately before callback construction.
        started_at: SystemTime,
        /// Observed time after completion or a caught panic.
        finished_at: SystemTime,
        /// Completion or the captured panic diagnostic.
        result: Result<(), String>,
        /// Authoritative completion, separate from ordinary return or error.
        outcome: Option<crate::execution::Outcome>,
    },
    /// The executor skipped a selected slot; no invocation is manufactured.
    Skipped {
        /// Time of the native decision, not callback execution.
        at: SystemTime,
        /// Reason this selected slot did not execute.
        reason: ScenarioHookSkipReason,
    },
}
/// One selected callback slot's actual decision in one native attempt.
#[derive(Clone, Debug)]
pub struct ScenarioHookRecord {
    /// Actual native attempt.
    pub attempt: ScenarioHookAttempt,
    /// Stable selected slot and its registration metadata.
    pub definition: ScenarioHookDefinition,
    /// Actual invocation or explicit skipped decision.
    pub disposition: ScenarioHookDisposition,
}
/// Native World initialization before an observed setup stage's callbacks.
#[derive(Clone, Debug)]
pub struct ScenarioHookInitialization {
    /// Attempt whose World was initialized.
    pub attempt: ScenarioHookAttempt,
    /// Observed construction start.
    pub started_at: SystemTime,
    /// Observed construction completion or failure.
    pub finished_at: SystemTime,
    /// Actual constructor result; this is not a callback result.
    pub result: Result<(), String>,
}
#[derive(Debug, Default)]
struct State {
    plans: Vec<ScenarioHookPlan>,
    records: Vec<ScenarioHookRecord>,
    initializations: Vec<ScenarioHookInitialization>,
}
/// Explicit shared observation for one run; create a fresh store for each run.
///
/// Snapshots retain actual native identities. Consumers can join a selected
/// plan
/// using its URI, Rule/scenario value and attempt, then use its ScenarioId for
/// records. No mutable global current-scenario pointer is used.
#[derive(Clone, Debug, Default)]
pub struct ScenarioHookStore(Arc<Mutex<State>>);
impl ScenarioHookStore {
    /// Creates a fresh store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Snapshots all explicit selection plans, including empty stage
    /// selections.
    #[must_use]
    pub fn plans(&self) -> Vec<ScenarioHookPlan> {
        self.0.lock().expect("scenario hook store poisoned").plans.clone()
    }
    /// Snapshots completed callback decisions.
    #[must_use]
    pub fn records(&self) -> Vec<ScenarioHookRecord> {
        self.0.lock().expect("scenario hook store poisoned").records.clone()
    }
    /// Snapshots constructor observations, separate from callback invocations.
    #[must_use]
    pub fn initializations(&self) -> Vec<ScenarioHookInitialization> {
        self.0
            .lock()
            .expect("scenario hook store poisoned")
            .initializations
            .clone()
    }
    /// Locates a native selection plan using source-backed scenario identity.
    #[must_use]
    pub fn plan_for(
        &self,
        uri: &Path,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        attempt: usize,
    ) -> Option<ScenarioHookPlan> {
        self.0
            .lock()
            .expect("scenario hook store poisoned")
            .plans
            .iter()
            .find(|p| {
                p.attempt.feature.path.as_deref() == Some(uri)
                    && crate::feature::same_rule_identity(
                        p.attempt.rule.as_deref(),
                        rule,
                    )
                    && p.attempt.scenario.as_ref() == scenario
                    && p.attempt.attempt == attempt
            })
            .cloned()
    }
    /// Snapshots callback decisions for one unique runner attempt identity.
    #[must_use]
    pub fn records_for(&self, id: ScenarioId) -> Vec<ScenarioHookRecord> {
        self.records()
            .into_iter()
            .filter(|r| r.attempt.scenario_id == id)
            .collect()
    }
    pub(crate) fn plan(&self, plan: ScenarioHookPlan) {
        self.0.lock().expect("scenario hook store poisoned").plans.push(plan);
    }
    pub(crate) fn push(&self, record: ScenarioHookRecord) {
        self.0
            .lock()
            .expect("scenario hook store poisoned")
            .records
            .push(record);
    }
    pub(crate) fn initialized(&self, record: ScenarioHookInitialization) {
        self.0
            .lock()
            .expect("scenario hook store poisoned")
            .initializations
            .push(record);
    }
    pub(crate) fn forget_attempt(&self, id: ScenarioId) {
        let mut state = self.0.lock().expect("scenario hook store poisoned");
        state.plans.retain(|p| p.attempt.scenario_id != id);
        state.records.retain(|r| r.attempt.scenario_id != id);
        state.initializations.retain(|r| r.attempt.scenario_id != id);
    }
    pub(crate) fn failures(&self, id: ScenarioId) -> Vec<ScenarioHookFailure> {
        self.records_for(id)
            .into_iter()
            .filter_map(|r| match r.disposition {
                ScenarioHookDisposition::Executed {
                    result: Err(message),
                    ..
                } => Some(ScenarioHookFailure {
                    definition: r.definition,
                    message,
                }),
                _ => None,
            })
            .collect()
    }
}
pub(crate) fn next_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(0);
    ID.fetch_add(1, Ordering::Relaxed)
}
