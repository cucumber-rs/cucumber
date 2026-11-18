//! Ordered global callbacks for one native executor invocation.
//!
//! All setup callbacks run; scenarios run only if every setup callback passed.
//! All cleanup callbacks run in reverse order, including after setup failure.
//! This is the single-executor Cucumber.js 13.3 contract, not process workers.

use futures::future::LocalBoxFuture;
use std::{
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

/// Global lifecycle stage for a registered callback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunHookKind {
    /// Setup before any scenarios execute.
    BeforeAll,
    /// Cleanup after scenarios, or after setup prevented scenarios.
    AfterAll,
}
/// Stable identity and name of a registered global callback.
#[derive(Clone, Debug)]
pub struct RunHookDefinition {
    /// Process-local registration identity retained by registry clones.
    pub id: u64,
    /// Caller-supplied callback name.
    pub name: String,
    /// Setup or cleanup stage.
    pub kind: RunHookKind,
}
/// One global callback that actually ran, including its observed result.
#[derive(Clone, Debug)]
pub struct RunHookRecord {
    /// Unique identity allocated when this invocation starts.
    pub invocation_id: u64,
    /// Native executor invocation containing this callback.
    pub run_id: u64,
    /// Registration actually invoked.
    pub definition: RunHookDefinition,
    /// Observed time before callback construction and polling.
    pub started_at: SystemTime,
    /// Observed completion or caught-panic time.
    pub finished_at: SystemTime,
    /// Successful completion or the captured panic message.
    pub result: Result<(), String>,
}
/// A selected global lifecycle slot skipped without constructing its callback.
#[derive(Clone, Debug)]
pub struct RunHookSkip {
    /// Unique decision identity; this is not a callback invocation.
    pub decision_id: u64,
    /// Native executor invocation containing this decision.
    pub run_id: u64,
    /// Actual selected registration.
    pub definition: RunHookDefinition,
    /// Decision timestamp used for a zero-duration skipped lifecycle.
    pub at: SystemTime,
}
/// Identity passed to a global callback, without a scenario World.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RunHookScope {
    /// Native executor invocation containing this callback.
    pub run_id: u64,
    /// Unique identity allocated for this callback invocation.
    pub invocation_id: u64,
    /// Registration currently executing.
    pub definition: RunHookDefinition,
}
#[derive(Debug, Default)]
struct State {
    current: Option<u64>,
    records: Vec<RunHookRecord>,
    skips: Vec<RunHookSkip>,
}
/// Actual global callback records for explicit observation/report integrations.
///
/// Use a fresh store for concurrent independent runs. Sequential reuse retains
/// previous records, separated by run IDs. Completed writer statistics retain
/// their own run ID and do not change when a later run uses the same store.
#[derive(Clone, Debug, Default)]
pub struct RunHookStore(Arc<Mutex<State>>);
impl RunHookStore {
    /// Creates a fresh store for one independent run lifecycle.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns the most recently started native run identity.
    #[must_use]
    pub fn current_run(&self) -> Option<u64> {
        self.0.lock().expect("run hook store poisoned").current
    }
    /// Returns a snapshot of all completed invocations retained by this store.
    #[must_use]
    pub fn records(&self) -> Vec<RunHookRecord> {
        self.0.lock().expect("run hook store poisoned").records.clone()
    }
    /// Returns completed callback records for the specified native run.
    #[must_use]
    pub fn records_for(&self, run_id: u64) -> Vec<RunHookRecord> {
        self.0
            .lock()
            .expect("run hook store poisoned")
            .records
            .iter()
            .filter(|r| r.run_id == run_id)
            .cloned()
            .collect()
    }
    /// Returns all skipped selections without implying any callback invocation.
    #[must_use]
    pub fn skips(&self) -> Vec<RunHookSkip> {
        self.0.lock().expect("run hook store poisoned").skips.clone()
    }
    /// Returns skipped global selections for one native run, in decision order.
    #[must_use]
    pub fn skipped_for(&self, run_id: u64) -> Vec<RunHookSkip> {
        self.0
            .lock()
            .expect("run hook store poisoned")
            .skips
            .iter()
            .filter(|r| r.run_id == run_id)
            .cloned()
            .collect()
    }
    /// Counts failed callback invocations for the specified native run.
    #[must_use]
    pub fn failures_for(&self, run_id: u64) -> usize {
        self.0
            .lock()
            .expect("run hook store poisoned")
            .records
            .iter()
            .filter(|r| r.run_id == run_id && r.result.is_err())
            .count()
    }
    fn begin(&self) -> u64 {
        let id = next_id();
        self.0.lock().expect("run hook store poisoned").current = Some(id);
        id
    }
    fn push(&self, record: RunHookRecord) {
        self.0.lock().expect("run hook store poisoned").records.push(record);
    }
}
type Callback = Box<dyn for<'a> Fn(&'a RunHookScope) -> LocalBoxFuture<'a, ()>>;
struct Registered {
    definition: RunHookDefinition,
    callback: Callback,
    timeout: Option<super::TimeoutPolicy>,
}
/// Named callbacks that wrap all scenarios in one native run.
///
/// Callbacks receive no scenario World. Capture shared application configuration
/// explicitly in their closures. Panics from construction or polling are recorded
/// individually and do not prevent subsequent callbacks from running. This
/// cleanup guarantee applies to awaited runs; dropping/cancelling the runner
/// future cannot synchronously execute asynchronous cleanup.
#[derive(Clone, Default)]
pub struct RunHooks {
    before: Vec<Arc<Registered>>,
    after: Vec<Arc<Registered>>,
    store: RunHookStore,
}
impl fmt::Debug for RunHooks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunHooks")
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
impl RunHooks {
    /// Creates an empty registry with a fresh observation store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Shares actual callback records with a reporting integration.
    #[must_use]
    pub fn with_store(mut self, store: RunHookStore) -> Self {
        self.store = store;
        self
    }
    /// Returns a clone of this registry's observation store.
    #[must_use]
    pub fn store(&self) -> RunHookStore {
        self.store.clone()
    }
    /// Lists registrations without claiming they have executed.
    #[must_use]
    pub fn definitions(&self) -> Vec<RunHookDefinition> {
        self.before
            .iter()
            .chain(&self.after)
            .map(|h| h.definition.clone())
            .collect()
    }
    /// Registers global setup in registration order; failures do not stop later setup.
    #[must_use]
    pub fn before<F>(mut self, name: impl Into<String>, callback: F) -> Self
    where
        F: for<'a> Fn(&'a RunHookScope) -> LocalBoxFuture<'a, ()> + 'static,
    {
        self.before.push(Arc::new(Registered {
            definition: RunHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: RunHookKind::BeforeAll,
            },
            callback: Box::new(callback),
            timeout: None,
        }));
        self
    }
    /// Registers global cleanup, invoked in reverse order despite earlier failures.
    #[must_use]
    pub fn after<F>(mut self, name: impl Into<String>, callback: F) -> Self
    where
        F: for<'a> Fn(&'a RunHookScope) -> LocalBoxFuture<'a, ()> + 'static,
    {
        self.after.push(Arc::new(Registered {
            definition: RunHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: RunHookKind::AfterAll,
            },
            callback: Box::new(callback),
            timeout: None,
        }));
        self
    }
    /// Registers a global callback with an independent deadline override.
    #[must_use]
    pub fn before_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(&'a RunHookScope) -> LocalBoxFuture<'a, ()> + 'static,
    {
        let mut hooks = self.before(name, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers a global callback with an independent deadline override.
    #[must_use]
    pub fn after_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(&'a RunHookScope) -> LocalBoxFuture<'a, ()> + 'static,
    {
        let mut hooks = self.after(name, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    pub(crate) fn skip_before(&self, run_id: u64) {
        for hook in &self.before {
            self.skip(hook, run_id);
        }
    }
    pub(crate) fn skip_after(&self, run_id: u64) {
        for hook in self.after.iter().rev() {
            self.skip(hook, run_id);
        }
    }
    fn skip(&self, hook: &Registered, run_id: u64) {
        self.store.0.lock().expect("run hook store poisoned").skips.push(
            RunHookSkip {
                decision_id: next_id(),
                run_id,
                definition: hook.definition.clone(),
                at: SystemTime::now(),
            },
        );
    }
    pub(crate) fn begin_run(&self) -> u64 {
        self.store.begin()
    }
    async fn invoke(
        &self,
        hook: &Registered,
        run_id: u64,
        timeouts: &super::Timeouts,
    ) -> bool {
        let invocation_id = next_id();
        let started_at = SystemTime::now();
        let scope = RunHookScope {
            run_id,
            invocation_id,
            definition: hook.definition.clone(),
        };
        let result = timeouts
            .invoke_callback(|| (hook.callback)(&scope), hook.timeout)
            .await
            .map_err(|e| super::step_hooks::panic_message(e.as_ref()));
        let passed = result.is_ok();
        self.store.push(RunHookRecord {
            invocation_id,
            run_id,
            definition: hook.definition.clone(),
            started_at,
            finished_at: SystemTime::now(),
            result,
        });
        passed
    }
    pub(crate) async fn run_before(
        &self,
        run_id: u64,
        timeouts: &super::Timeouts,
    ) -> bool {
        let mut passed = true;
        for hook in &self.before {
            passed &= self.invoke(hook, run_id, timeouts).await;
        }
        passed
    }
    pub(crate) async fn run_after(
        &self,
        run_id: u64,
        timeouts: &super::Timeouts,
    ) -> bool {
        let mut passed = true;
        for hook in self.after.iter().rev() {
            passed &= self.invoke(hook, run_id, timeouts).await;
        }
        passed
    }
}
