//! Opt-in logical-case eligibility and observations of real dispatcher decisions.
//!
//! Reservations include retry delay. This is not worker/process isolation or an
//! external lease guard: an entirely idle dispatcher deliberately falls back.
use super::ScenarioType;
use crate::event::Source;
use std::{
    collections::HashMap,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

/// Immutable selected native case presented to an assignment predicate.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AssignmentCase {
    /// Logical identity within this assignment run, retained through every retry.
    pub case_id: u64,
    /// Position after the configured global permutation, before queue removals.
    pub plan_index: usize,
    /// Actual parsed containing feature.
    pub feature: Source<gherkin::Feature>,
    /// Actual containing Rule, when present.
    pub rule: Option<Source<gherkin::Rule>>,
    /// Actual expanded native scenario, including inherited Examples tags.
    pub scenario: Source<gherkin::Scenario>,
    /// Mandatory native concurrency or Serial classification.
    pub kind: ScenarioType,
}
/// Why a selected logical case was admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentAdmission {
    /// The predicate accepted this candidate against the actual running set.
    Eligible,
    /// All accessible candidates were rejected while no logical case was assigned.
    IdleFallback,
}
/// Why the dispatcher stopped filling available capacity and awaited progress.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentWait {
    /// No accessible candidate was accepted against the current reservations.
    NoEligible,
    /// A queued Serial barrier must wait for existing reservations to release.
    SerialBarrier,
}
/// Aggregate warning emitted once on normal completion after idle interventions.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AssignmentWarning {
    /// Native assignment run that made the interventions.
    pub run_id: u64,
    /// Caller-supplied policy name.
    pub policy: String,
    /// Number of actual idle-head fallback admissions.
    pub idle_interventions: usize,
    /// Human-readable diagnostic sent to stderr or the configured warning sink.
    pub message: String,
}
/// An observed dispatcher action; none of these are callback/body invocations.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum AssignmentDecisionKind {
    /// A predicate returned a decision for this exact snapshot.
    Evaluated {
        /// Actual selected candidate.
        candidate: AssignmentCase,
        /// Assigned logical cases in admission order, including retry delays.
        running: Vec<AssignmentCase>,
        /// Actual predicate return value.
        eligible: bool,
    },
    /// A logical reservation was installed before dispatching its first attempt.
    Admitted {
        /// Actual admitted case.
        case: AssignmentCase,
        /// Predicate acceptance or explicitly observed idle fallback.
        reason: AssignmentAdmission,
    },
    /// The entire logical retry-loop future completed and released its reservation.
    Released {
        /// Logical identity that was released.
        case_id: u64,
    },
    /// The dispatcher transitioned into a wait for real progress.
    Waiting {
        /// Actual scheduling reason.
        reason: AssignmentWait,
        /// Assigned cases that still hold reservations.
        running: Vec<AssignmentCase>,
    },
    /// Completed-run aggregate warning, also delivered to the warning sink.
    Warning(AssignmentWarning),
}
/// One actual assignment action with stable run and sequence identity.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AssignmentDecision {
    /// Assignment run owning this decision.
    pub run_id: u64,
    /// Zero-based sequence within that run.
    pub sequence: usize,
    /// Observed time of the dispatcher action.
    pub at: SystemTime,
    /// Actual evaluation, reservation change, wait or warning.
    pub kind: AssignmentDecisionKind,
}
#[derive(Debug, Default)]
struct State {
    current: Option<u64>,
    next: HashMap<u64, usize>,
    records: Vec<AssignmentDecision>,
}
/// Shared typed observations, without altering native event enums or Messages.
///
/// Use a fresh store for independent concurrent runs. Sequential reuse retains
/// prior records separated by run IDs. Unchanged blocked states are not polled
/// and do not append repeated waiting/evaluation records.
#[derive(Clone, Debug, Default)]
pub struct AssignmentStore(Arc<Mutex<State>>);
impl AssignmentStore {
    /// Creates an empty assignment observation store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns the latest started assignment run, or None when not activated.
    #[must_use]
    pub fn current_run(&self) -> Option<u64> {
        self.0.lock().expect("assignment store poisoned").current
    }
    /// Returns a snapshot of every actual retained dispatcher decision.
    #[must_use]
    pub fn records(&self) -> Vec<AssignmentDecision> {
        self.0.lock().expect("assignment store poisoned").records.clone()
    }
    /// Returns actual dispatcher decisions for a specific assignment run.
    #[must_use]
    pub fn records_for(&self, run_id: u64) -> Vec<AssignmentDecision> {
        self.0
            .lock()
            .expect("assignment store poisoned")
            .records
            .iter()
            .filter(|r| r.run_id == run_id)
            .cloned()
            .collect()
    }
    /// Returns completed aggregate warnings for a specific assignment run.
    #[must_use]
    pub fn warnings_for(&self, run_id: u64) -> Vec<AssignmentWarning> {
        self.records_for(run_id)
            .into_iter()
            .filter_map(|r| match r.kind {
                AssignmentDecisionKind::Warning(w) => Some(w),
                _ => None,
            })
            .collect()
    }
    fn begin(&self) -> u64 {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut state = self.0.lock().expect("assignment store poisoned");
        state.current = Some(id);
        state.next.insert(id, 0);
        id
    }
    fn push(&self, run_id: u64, kind: AssignmentDecisionKind) {
        let mut state = self.0.lock().expect("assignment store poisoned");
        let next = state.next.get_mut(&run_id).expect("assignment run started");
        let sequence = *next;
        *next += 1;
        state.records.push(AssignmentDecision {
            run_id,
            sequence,
            at: SystemTime::now(),
            kind,
        });
    }
}
type Predicate = dyn Fn(&AssignmentCase, &[AssignmentCase]) -> bool;
type WarningSink = dyn Fn(&AssignmentWarning);
/// Named synchronous eligibility policy for the explicit ordered dispatcher.
///
/// Treat the predicate as a pure function of candidate, assigned logical cases
/// and stable captured configuration. An external state change cannot wake this
/// dispatcher. Predicate panics propagate and are never treated as rejection.
/// Idle fallback guarantees progress, so this is not a security or lease guard.
#[derive(Clone)]
pub struct AssignmentPolicy {
    name: String,
    predicate: Arc<Predicate>,
    warning: Option<Arc<WarningSink>>,
    store: AssignmentStore,
}
impl fmt::Debug for AssignmentPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssignmentPolicy")
            .field("name", &self.name)
            .field("store", &self.store)
            .finish_non_exhaustive()
    }
}
impl AssignmentPolicy {
    /// Creates an explicit policy with a fresh store and aggregate stderr warnings.
    #[must_use]
    pub fn new<F>(name: impl Into<String>, predicate: F) -> Self
    where
        F: Fn(&AssignmentCase, &[AssignmentCase]) -> bool + 'static,
    {
        Self {
            name: name.into(),
            predicate: Arc::new(predicate),
            warning: None,
            store: AssignmentStore::new(),
        }
    }
    /// Shares real dispatcher observations with an application or reporter.
    #[must_use]
    pub fn with_store(mut self, store: AssignmentStore) -> Self {
        self.store = store;
        self
    }
    /// Returns the shared observation store.
    #[must_use]
    pub fn store(&self) -> AssignmentStore {
        self.store.clone()
    }
    /// Replaces default stderr warning output with an application-owned sink.
    /// Warnings remain in the store and are emitted once on normal completion.
    #[must_use]
    pub fn on_warning<F>(mut self, handler: F) -> Self
    where
        F: Fn(&AssignmentWarning) + 'static,
    {
        self.warning = Some(Arc::new(handler));
        self
    }
    pub(crate) fn begin(&self) -> u64 {
        self.store.begin()
    }
    pub(crate) fn evaluate(
        &self,
        run_id: u64,
        candidate: &AssignmentCase,
        running: &[AssignmentCase],
    ) -> bool {
        let eligible = (self.predicate)(candidate, running);
        self.store.push(
            run_id,
            AssignmentDecisionKind::Evaluated {
                candidate: candidate.clone(),
                running: running.to_vec(),
                eligible,
            },
        );
        eligible
    }
    pub(crate) fn admitted(
        &self,
        run_id: u64,
        case: &AssignmentCase,
        reason: AssignmentAdmission,
    ) {
        self.store.push(
            run_id,
            AssignmentDecisionKind::Admitted { case: case.clone(), reason },
        );
    }
    pub(crate) fn released(&self, run_id: u64, case_id: u64) {
        self.store.push(run_id, AssignmentDecisionKind::Released { case_id });
    }
    pub(crate) fn waiting(
        &self,
        run_id: u64,
        reason: AssignmentWait,
        running: &[AssignmentCase],
    ) {
        self.store.push(
            run_id,
            AssignmentDecisionKind::Waiting {
                reason,
                running: running.to_vec(),
            },
        );
    }
    pub(crate) fn finish(&self, run_id: u64, count: usize) {
        if count == 0 {
            return;
        }
        let warning = AssignmentWarning {
            run_id,
            policy: self.name.clone(),
            idle_interventions: count,
            message: format!(
                "WARNING: assignment policy `{}` required {count} idle fallback(s); consider revising its predicate.",
                self.name
            ),
        };
        self.store
            .push(run_id, AssignmentDecisionKind::Warning(warning.clone()));
        if let Some(sink) = &self.warning {
            sink(&warning);
        } else {
            eprintln!("{}", warning.message);
        }
    }
}
