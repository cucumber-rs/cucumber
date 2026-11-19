//! Ordered per-step lifecycle callbacks and records of actual invocations.
//!
//! Step hooks follow the Cucumber.js 13.3 execution contract: all selected
//! setup
//! callbacks run, then the body runs unless setup failed, then all selected
//! cleanup callbacks run in reverse order. Cleanup sees the same body outcome.
//! In opt-in outcome mode, pending/skipped setup still permits the body.
//! Undefined, ambiguous, and unexecuted steps do not invoke these callbacks.

use std::{
    any::Any,
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use futures::future::LocalBoxFuture;
use gherkin::{Feature, Rule, Scenario, Step, tagexpr::TagOperation};

use super::basic::ScenarioId;
use crate::{event::Source, tag::Ext as _};

/// Which per-step lifecycle stage registered a callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepHookKind {
    /// Setup immediately before a resolved step body.
    Before,
    /// Cleanup after the body or after setup prevented the body.
    After,
}

/// The body result supplied to every after-step callback.
///
/// This is not the aggregate result of previous cleanup callbacks. `NotRun`
/// means that failed setup prevented execution of the resolved step body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepBodyOutcome {
    /// The body returned normally after explicitly marking work pending.
    Pending,
    /// The body returned normally after explicitly skipping its work.
    Skipped,
    /// Failed setup prevented the body from being invoked.
    NotRun,
    /// The step body completed without panicking.
    Passed,
    /// The step body panicked with this message.
    Failed(String),
}

/// Native source and execution identity of a resolved step invocation.
///
/// `Source` values retain the actual runner identity, including expanded
/// outline
/// rows. Each retry has a fresh `scenario_id`; `attempt` is its zero-based
/// index.
#[derive(Clone, Debug)]
pub struct StepHookScope {
    /// Actual runner source identity of the containing feature.
    pub feature: Source<Feature>,
    /// Actual containing Rule identity when the scenario belongs to a Rule.
    pub rule: Option<Source<Rule>>,
    /// Expanded scenario identity used by the running attempt.
    pub scenario: Source<Scenario>,
    /// Executing background or scenario step identity.
    pub step: Source<Step>,
    /// Unique native execution identity; each retry receives a new ID.
    pub scenario_id: ScenarioId,
    /// Zero-based retry attempt index.
    pub attempt: usize,
    /// Whether this step belongs to a feature or Rule Background.
    pub is_background: bool,
}

/// Stable registration identity and human-readable name within this process.
#[derive(Clone, Debug)]
pub struct StepHookDefinition {
    /// Process-local stable registration identifier, shared by registry clones.
    pub id: u64,
    /// Caller-supplied callback name used in failures and observations.
    pub name: String,
    /// Setup or cleanup stage for this registration.
    pub kind: StepHookKind,
    /// Optional inherited-tag condition selecting scenarios for this callback.
    pub tags: Option<TagOperation>,
}

/// One callback that actually ran, with its original scope, timing and result.
#[derive(Clone, Debug)]
pub struct StepHookRecord {
    /// Unique identifier allocated when this callback actually starts.
    pub invocation_id: u64,
    /// Registration that was invoked.
    pub definition: StepHookDefinition,
    /// Native scenario attempt and step associated with the invocation.
    pub scope: StepHookScope,
    /// Observed callback start time before future construction and polling.
    pub started_at: SystemTime,
    /// Observed time after callback completion or caught panic.
    pub finished_at: SystemTime,
    /// Successful completion or the callback panic message.
    pub result: Result<(), String>,
    /// Authoritative completion when explicit outcome semantics are enabled.
    pub outcome: Option<crate::execution::Outcome>,
}

/// Shared observation store for explicit lifecycle integrations.
///
/// Use a fresh store per independent run. Clones share records. Records are
/// appended when callbacks finish, so concurrent completion order can differ
/// from invocation order. IDs and timestamps retain those distinctions.
/// This store does not fabricate native events or Messages hook test steps.
#[derive(Clone, Debug, Default)]
pub struct StepHookStore(Arc<Mutex<Vec<StepHookRecord>>>);
impl StepHookStore {
    /// Creates a new empty observation store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns an immutable snapshot of completed callback records.
    #[must_use]
    pub fn records(&self) -> Vec<StepHookRecord> {
        self.0.lock().expect("step hook store poisoned").clone()
    }
    fn push(&self, record: StepHookRecord) {
        self.0.lock().expect("step hook store poisoned").push(record);
    }
}

type Before<W> =
    Box<dyn for<'a> Fn(&'a StepHookScope, &'a mut W) -> LocalBoxFuture<'a, ()>>;
type After<W> = Box<
    dyn for<'a> Fn(
        &'a StepHookScope,
        &'a StepBodyOutcome,
        &'a mut W,
    ) -> LocalBoxFuture<'a, ()>,
>;
struct Registered<F> {
    definition: StepHookDefinition,
    callback: F,
    timeout: Option<super::TimeoutPolicy>,
}
impl<F> Registered<F> {
    fn applies(&self, scope: &StepHookScope) -> bool {
        self.definition.tags.as_ref().is_none_or(|tags| {
            tags.eval(
                scope
                    .feature
                    .tags
                    .iter()
                    .chain(scope.rule.iter().flat_map(|rule| &rule.tags))
                    .chain(&scope.scenario.tags),
            )
        })
    }
}

/// Ordered, named setup and cleanup around resolved step bodies.
///
/// Before callbacks execute in registration order, including after another
/// before callback fails. After callbacks execute in reverse order and continue
/// after every panic. Failed setup skips the body; after callbacks still run.
/// Hook failures become an ordinary failed native step, preserving existing
/// writers and retry behavior. Exact per-callback results remain in the store.
/// Installing this registry does not replace scenario before/after hooks.
pub struct StepHooks<W> {
    before: Vec<Arc<Registered<Before<W>>>>,
    after: Vec<Arc<Registered<After<W>>>>,
    store: StepHookStore,
}
impl<W> Default for StepHooks<W> {
    fn default() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
            store: StepHookStore::new(),
        }
    }
}
impl<W> Clone for StepHooks<W> {
    fn clone(&self) -> Self {
        Self {
            before: self.before.clone(),
            after: self.after.clone(),
            store: self.store.clone(),
        }
    }
}
impl<W> fmt::Debug for StepHooks<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StepHooks")
            .field(
                "before",
                &self.before.iter().map(|h| &h.definition).collect::<Vec<_>>(),
            )
            .field(
                "after",
                &self.after.iter().map(|h| &h.definition).collect::<Vec<_>>(),
            )
            .finish()
    }
}
fn next_id() -> u64 {
    static ID: AtomicU64 = AtomicU64::new(0);
    ID.fetch_add(1, Ordering::Relaxed)
}
fn definition(
    name: impl Into<String>,
    kind: StepHookKind,
    tags: Option<TagOperation>,
) -> StepHookDefinition {
    StepHookDefinition { id: next_id(), name: name.into(), kind, tags }
}
impl<W: 'static> StepHooks<W> {
    /// Creates an empty step-hook registry with a fresh observation store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Shares actual callback records with an explicit reporting integration.
    #[must_use]
    pub fn with_store(mut self, store: StepHookStore) -> Self {
        self.store = store;
        self
    }
    /// Returns a clone of the shared callback observation store.
    #[must_use]
    pub fn store(&self) -> StepHookStore {
        self.store.clone()
    }
    /// Lists registrations without implying that any callback has executed.
    #[must_use]
    pub fn definitions(&self) -> Vec<StepHookDefinition> {
        self.before
            .iter()
            .map(|h| h.definition.clone())
            .chain(self.after.iter().map(|h| h.definition.clone()))
            .collect()
    }
    /// Registers unconditional setup in registration order.
    #[must_use]
    pub fn before<F>(mut self, name: impl Into<String>, callback: F) -> Self
    where
        F: for<'a> Fn(&'a StepHookScope, &'a mut W) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        self.before.push(Arc::new(Registered {
            definition: definition(name, StepHookKind::Before, None),
            callback: Box::new(callback),
            timeout: None,
        }));
        self
    }
    /// Registers setup selected by inherited feature, Rule and scenario tags.
    #[must_use]
    pub fn before_if<F>(
        mut self,
        name: impl Into<String>,
        tags: TagOperation,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(&'a StepHookScope, &'a mut W) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        self.before.push(Arc::new(Registered {
            definition: definition(name, StepHookKind::Before, Some(tags)),
            callback: Box::new(callback),
            timeout: None,
        }));
        self
    }
    /// Registers unconditional cleanup, invoked in reverse registration order.
    #[must_use]
    pub fn after<F>(mut self, name: impl Into<String>, callback: F) -> Self
    where
        F: for<'a> Fn(
                &'a StepHookScope,
                &'a StepBodyOutcome,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        self.after.push(Arc::new(Registered {
            definition: definition(name, StepHookKind::After, None),
            callback: Box::new(callback),
            timeout: None,
        }));
        self
    }
    /// Registers cleanup selected by inherited feature, Rule and scenario tags.
    #[must_use]
    pub fn after_if<F>(
        mut self,
        name: impl Into<String>,
        tags: TagOperation,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a StepHookScope,
                &'a StepBodyOutcome,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        self.after.push(Arc::new(Registered {
            definition: definition(name, StepHookKind::After, Some(tags)),
            callback: Box::new(callback),
            timeout: None,
        }));
        self
    }
    /// Registers a callback with an explicit independent deadline override.
    #[must_use]
    pub fn before_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(&'a StepHookScope, &'a mut W) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.before(name, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers a callback with an explicit independent deadline override.
    #[must_use]
    pub fn before_if_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: TagOperation,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(&'a StepHookScope, &'a mut W) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.before_if(name, tags, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers a callback with an explicit independent deadline override.
    #[must_use]
    pub fn after_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a StepHookScope,
                &'a StepBodyOutcome,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.after(name, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers a callback with an explicit independent deadline override.
    #[must_use]
    pub fn after_if_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: TagOperation,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a StepHookScope,
                &'a StepBodyOutcome,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.after_if(name, tags, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    pub(crate) async fn run_before(
        &self,
        scope: &StepHookScope,
        world: &mut W,
        timeouts: &super::Timeouts,
        attachments: &mut crate::attachment::ScopeGuard,
        execution: &crate::execution::ExecutionControl,
        policy: crate::execution::OutcomePolicy,
    ) -> super::outcomes::Summary {
        let mut summary = super::outcomes::Summary::default();
        for hook in &self.before {
            if !hook.applies(scope) {
                continue;
            }
            let invocation_id = next_id();
            let started_at = SystemTime::now();
            let control = execution.enter();
            let callback_world = &mut *world;
            let result = timeouts
                .invoke_callback(
                    move || (hook.callback)(scope, callback_world),
                    hook.timeout,
                )
                .await;
            let completion = control.finish();
            if result
                .as_ref()
                .is_err_and(|error| error.is::<super::TimeoutError>())
            {
                attachments.renew();
            }
            let result = result.map_err(|error| panic_message(error.as_ref()));
            let status = super::outcomes::status(&result, completion);
            summary.status = summary.status.max(status);
            if let Err(error) = &result {
                summary.failures.push(format!(
                    "BeforeStep `{}`: {error}",
                    hook.definition.name
                ));
            }
            self.store.push(StepHookRecord {
                invocation_id,
                definition: hook.definition.clone(),
                scope: scope.clone(),
                started_at,
                finished_at: SystemTime::now(),
                result,
                outcome: super::outcomes::metadata(policy, status),
            });
        }
        summary
    }
    pub(crate) async fn run_after(
        &self,
        scope: &StepHookScope,
        body: &StepBodyOutcome,
        world: &mut W,
        timeouts: &super::Timeouts,
        attachments: &mut crate::attachment::ScopeGuard,
        execution: &crate::execution::ExecutionControl,
        policy: crate::execution::OutcomePolicy,
    ) -> super::outcomes::Summary {
        let mut summary = super::outcomes::Summary::default();
        for hook in self.after.iter().rev() {
            if !hook.applies(scope) {
                continue;
            }
            let invocation_id = next_id();
            let started_at = SystemTime::now();
            let control = execution.enter();
            let callback_world = &mut *world;
            let result = timeouts
                .invoke_callback(
                    move || (hook.callback)(scope, body, callback_world),
                    hook.timeout,
                )
                .await;
            let completion = control.finish();
            if result
                .as_ref()
                .is_err_and(|error| error.is::<super::TimeoutError>())
            {
                attachments.renew();
            }
            let result = result.map_err(|error| panic_message(error.as_ref()));
            let status = super::outcomes::status(&result, completion);
            summary.status = summary.status.max(status);
            if let Err(error) = &result {
                summary.failures.push(format!(
                    "AfterStep `{}`: {error}",
                    hook.definition.name
                ));
            }
            self.store.push(StepHookRecord {
                invocation_id,
                definition: hook.definition.clone(),
                scope: scope.clone(),
                started_at,
                finished_at: SystemTime::now(),
                result,
                outcome: super::outcomes::metadata(policy, status),
            });
        }
        summary
    }
}
pub(crate) fn panic_message(error: &(dyn Any + Send)) -> String {
    error
        .downcast_ref::<super::TimeoutError>()
        .map(ToString::to_string)
        .or_else(|| error.downcast_ref::<String>().cloned())
        .or_else(|| error.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_else(|| "non-string panic payload".into())
}
