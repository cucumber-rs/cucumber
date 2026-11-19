//! Reusable ordered scenario callbacks with optional individual observation.
//!
//! Legacy callbacks retain their signatures and pre-cleanup snapshot. Scoped
//! callbacks additionally observe native attempt identity and evolving cleanup.
use std::{fmt, panic::Location, sync::Arc, time::SystemTime};

use futures::future::LocalBoxFuture;
use gherkin::{Feature, Rule, Scenario, tagexpr::TagOperation};

use super::scenario_hooks::*;
use crate::{event::ScenarioFinished, tag::Ext as _};

/// A legacy before-scenario callback.
pub type Before<W> = Box<
    dyn for<'a> Fn(
        &'a Feature,
        Option<&'a Rule>,
        &'a Scenario,
        &'a mut W,
    ) -> LocalBoxFuture<'a, ()>,
>;
/// A legacy after-scenario callback, retaining the pre-cleanup result.
pub type After<W> = Box<
    dyn for<'a> Fn(
        &'a Feature,
        Option<&'a Rule>,
        &'a Scenario,
        &'a ScenarioFinished,
        Option<&'a mut W>,
    ) -> LocalBoxFuture<'a, ()>,
>;
type ScopedBefore<W> = Box<
    dyn for<'a> Fn(&'a ScenarioHookScope, &'a mut W) -> LocalBoxFuture<'a, ()>,
>;
type ScopedAfter<W> = Box<
    dyn for<'a> Fn(
        &'a ScenarioHookScope,
        Option<&'a mut W>,
    ) -> LocalBoxFuture<'a, ()>,
>;
enum BeforeCallback<W> {
    Legacy(Before<W>),
    Scoped(ScopedBefore<W>),
}
enum AfterCallback<W> {
    Legacy(After<W>),
    Scoped(ScopedAfter<W>),
}
struct Registered<F> {
    definition: ScenarioHookDefinition,
    tags: Option<TagFilter>,
    callback: F,
    timeout: Option<super::TimeoutPolicy>,
}
enum TagFilter {
    Legacy(TagOperation),
    Official(crate::tag::Expression),
}
impl<F> Registered<F> {
    fn applies(&self, f: &Feature, r: Option<&Rule>, s: &Scenario) -> bool {
        self.tags.as_ref().is_none_or(|tags| match tags {
            TagFilter::Legacy(tags) => tags.eval(
                f.tags
                    .iter()
                    .chain(r.into_iter().flat_map(|r| &r.tags))
                    .chain(&s.tags),
            ),
            TagFilter::Official(tags) => tags.matches(f, r, s),
        })
    }
}
/// Named setup and reverse cleanup, preserving native stage event
/// compatibility.
///
/// Setup stops after failure; cleanup continues after each panic. Legacy After
/// callbacks keep the original ScenarioFinished. Additive scoped After
/// callbacks
/// also receive evolving native outcome and individual earlier callback
/// failures.
/// Only with_store enables individual attachment targets; without it, fresh
/// callback handles retain composite before/after targets for existing
/// reporters.
pub struct Hooks<W> {
    before: Vec<Arc<Registered<BeforeCallback<W>>>>,
    after: Vec<Arc<Registered<AfterCallback<W>>>>,
    store: ScenarioHookStore,
    observed: bool,
    empty: EmptyScenarioHooks,
}
impl<W> Default for Hooks<W> {
    fn default() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
            store: ScenarioHookStore::new(),
            observed: false,
            empty: EmptyScenarioHooks::Native,
        }
    }
}
impl<W> Clone for Hooks<W> {
    fn clone(&self) -> Self {
        Self {
            before: self.before.clone(),
            after: self.after.clone(),
            store: self.store.clone(),
            observed: self.observed,
            empty: self.empty,
        }
    }
}
impl<W> fmt::Debug for Hooks<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hooks")
            .field(
                "before",
                &self.before.iter().map(|h| &h.definition).collect::<Vec<_>>(),
            )
            .field(
                "after",
                &self.after.iter().map(|h| &h.definition).collect::<Vec<_>>(),
            )
            .field("observed", &self.observed)
            .field("empty", &self.empty)
            .finish()
    }
}
impl<W: 'static> Hooks<W> {
    /// Creates an empty registry. Empty stages preserve lazy World
    /// initialization.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Enables shared invocation observation and per-callback attachment
    /// targets.
    /// Configure a compatible Messages writer with this same store.
    #[must_use]
    pub fn with_store(mut self, store: ScenarioHookStore) -> Self {
        self.store = store;
        self.observed = true;
        self
    }
    /// Returns the explicitly enabled reporting store, if any.
    #[must_use]
    pub fn store(&self) -> Option<ScenarioHookStore> {
        self.observed.then(|| self.store.clone())
    }
    /// Snapshots registrations without claiming any callback has executed.
    #[must_use]
    pub fn definitions(&self) -> Vec<ScenarioHookDefinition> {
        self.before
            .iter()
            .map(|h| h.definition.clone())
            .chain(self.after.iter().map(|h| h.definition.clone()))
            .collect()
    }
    /// Selects explicit empty-plan behavior; native cleanup remains the
    /// default.
    #[must_use]
    pub fn empty_scenario_policy(mut self, policy: EmptyScenarioHooks) -> Self {
        self.empty = policy;
        self
    }
    /// Registers legacy setup in registration order.
    #[must_use]
    #[track_caller]
    pub fn before<F>(mut self, name: impl Into<String>, callback: F) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.before.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::Before,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: None,
            timeout: None,
            callback: BeforeCallback::Legacy(Box::new(callback)),
        }));
        self
    }
    /// Registers legacy setup in registration order selected by inherited tags.
    #[must_use]
    #[track_caller]
    pub fn before_if<F>(
        mut self,
        name: impl Into<String>,
        tags: TagOperation,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.before.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::Before,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: Some(TagFilter::Legacy(tags)),
            timeout: None,
            callback: BeforeCallback::Legacy(Box::new(callback)),
        }));
        self
    }
    /// Registers legacy setup in registration order selected by inherited tags.
    #[must_use]
    #[track_caller]
    pub fn before_matching<F>(
        mut self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.before.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::Before,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: Some(TagFilter::Official(tags)),
            timeout: None,
            callback: BeforeCallback::Legacy(Box::new(callback)),
        }));
        self
    }
    /// Registers scoped setup in registration order.
    #[must_use]
    #[track_caller]
    pub fn before_scoped<F>(
        mut self,
        name: impl Into<String>,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.before.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::Before,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: None,
            timeout: None,
            callback: BeforeCallback::Scoped(Box::new(callback)),
        }));
        self
    }
    /// Registers scoped setup in registration order selected by inherited tags.
    #[must_use]
    #[track_caller]
    pub fn before_scoped_matching<F>(
        mut self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.before.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::Before,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: Some(TagFilter::Official(tags)),
            timeout: None,
            callback: BeforeCallback::Scoped(Box::new(callback)),
        }));
        self
    }
    /// Registers legacy cleanup in reverse registration order.
    #[must_use]
    #[track_caller]
    pub fn after<F>(mut self, name: impl Into<String>, callback: F) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.after.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::After,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: None,
            timeout: None,
            callback: AfterCallback::Legacy(Box::new(callback)),
        }));
        self
    }
    /// Registers legacy cleanup in reverse registration order selected by
    /// inherited tags.
    #[must_use]
    #[track_caller]
    pub fn after_if<F>(
        mut self,
        name: impl Into<String>,
        tags: TagOperation,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.after.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::After,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: Some(TagFilter::Legacy(tags)),
            timeout: None,
            callback: AfterCallback::Legacy(Box::new(callback)),
        }));
        self
    }
    /// Registers legacy cleanup in reverse registration order selected by
    /// inherited tags.
    #[must_use]
    #[track_caller]
    pub fn after_matching<F>(
        mut self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.after.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::After,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: Some(TagFilter::Official(tags)),
            timeout: None,
            callback: AfterCallback::Legacy(Box::new(callback)),
        }));
        self
    }
    /// Registers scoped cleanup in reverse registration order.
    #[must_use]
    #[track_caller]
    pub fn after_scoped<F>(
        mut self,
        name: impl Into<String>,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.after.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::After,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: None,
            timeout: None,
            callback: AfterCallback::Scoped(Box::new(callback)),
        }));
        self
    }
    /// Registers scoped cleanup in reverse registration order selected by
    /// inherited tags.
    #[must_use]
    #[track_caller]
    pub fn after_scoped_matching<F>(
        mut self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let location = Location::caller();
        self.after.push(Arc::new(Registered {
            definition: ScenarioHookDefinition {
                id: next_id(),
                name: name.into(),
                kind: ScenarioHookKind::After,
                location: crate::step::Location {
                    path: location.file().into(),
                    line: location.line(),
                    column: location.column(),
                },
            },
            tags: Some(TagFilter::Official(tags)),
            timeout: None,
            callback: AfterCallback::Scoped(Box::new(callback)),
        }));
        self
    }

    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn before_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.before(name, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn before_if_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: TagOperation,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.before_if(name, tags, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn before_matching_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.before_matching(name, tags, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn before_scoped_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.before_scoped(name, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn before_scoped_matching_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                &'a mut W,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.before_scoped_matching(name, tags, callback);
        Arc::get_mut(hooks.before.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn after_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.after(name, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn after_if_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: TagOperation,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.after_if(name, tags, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn after_matching_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a Feature,
                Option<&'a Rule>,
                &'a Scenario,
                &'a ScenarioFinished,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.after_matching(name, tags, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn after_scoped_with_timeout<F>(
        self,
        name: impl Into<String>,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.after_scoped(name, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }
    /// Registers this callback with an explicit per-invocation deadline
    /// override.
    #[must_use]
    #[track_caller]
    pub fn after_scoped_matching_with_timeout<F>(
        self,
        name: impl Into<String>,
        tags: crate::tag::Expression,
        timeout: super::TimeoutPolicy,
        callback: F,
    ) -> Self
    where
        F: for<'a> Fn(
                &'a ScenarioHookScope,
                Option<&'a mut W>,
            ) -> LocalBoxFuture<'a, ()>
            + 'static,
    {
        let mut hooks = self.after_scoped_matching(name, tags, callback);
        Arc::get_mut(hooks.after.last_mut().expect("new registration"))
            .expect("new registration is uniquely owned")
            .timeout = Some(timeout);
        hooks
    }

    pub(crate) fn finish_attempt(&self, id: super::basic::ScenarioId) {
        if !self.observed {
            self.store.forget_attempt(id);
        }
    }
    pub(crate) fn begin_attempt(&self, attempt: ScenarioHookAttempt) {
        let selected = |h: &ScenarioHookDefinition| h.clone();
        self.store.plan(ScenarioHookPlan {
            before_stage: self.has_before(),
            after_stage: self.has_after(),
            before: self
                .before
                .iter()
                .filter(|h| {
                    h.applies(
                        &attempt.feature,
                        attempt.rule.as_deref(),
                        &attempt.scenario,
                    )
                })
                .map(|h| selected(&h.definition))
                .collect(),
            after: self
                .after
                .iter()
                .rev()
                .filter(|h| {
                    h.applies(
                        &attempt.feature,
                        attempt.rule.as_deref(),
                        &attempt.scenario,
                    )
                })
                .map(|h| selected(&h.definition))
                .collect(),
            attempt,
        });
    }
    pub(crate) fn initialized(
        &self,
        attempt: &ScenarioHookAttempt,
        started_at: SystemTime,
        result: Result<(), String>,
    ) {
        self.store.initialized(ScenarioHookInitialization {
            attempt: attempt.clone(),
            started_at,
            finished_at: SystemTime::now(),
            result: result.clone(),
        });
        if result.is_err() {
            for h in self.before.iter().filter(|h| {
                h.applies(
                    &attempt.feature,
                    attempt.rule.as_deref(),
                    &attempt.scenario,
                )
            }) {
                self.skip(
                    attempt,
                    &h.definition,
                    ScenarioHookSkipReason::WorldInitializationFailed,
                );
            }
        }
    }
    pub(crate) fn skip_stage(
        &self,
        attempt: &ScenarioHookAttempt,
        kind: ScenarioHookKind,
        reason: ScenarioHookSkipReason,
    ) {
        let definitions: Vec<_> = match kind {
            ScenarioHookKind::Before => self
                .before
                .iter()
                .filter(|h| {
                    h.applies(
                        &attempt.feature,
                        attempt.rule.as_deref(),
                        &attempt.scenario,
                    )
                })
                .map(|h| &h.definition)
                .collect(),
            ScenarioHookKind::After => self
                .after
                .iter()
                .rev()
                .filter(|h| {
                    h.applies(
                        &attempt.feature,
                        attempt.rule.as_deref(),
                        &attempt.scenario,
                    )
                })
                .map(|h| &h.definition)
                .collect(),
        };
        for definition in definitions {
            self.skip(attempt, definition, reason);
        }
    }
    fn skip(
        &self,
        attempt: &ScenarioHookAttempt,
        definition: &ScenarioHookDefinition,
        reason: ScenarioHookSkipReason,
    ) {
        self.store.push(ScenarioHookRecord {
            attempt: attempt.clone(),
            definition: definition.clone(),
            disposition: ScenarioHookDisposition::Skipped {
                at: SystemTime::now(),
                reason,
            },
        });
    }
    fn target(
        &self,
        definition: &ScenarioHookDefinition,
    ) -> crate::attachment::Target {
        if self.observed {
            crate::attachment::Target::ScenarioHook(definition.id)
        } else {
            match definition.kind {
                ScenarioHookKind::Before => {
                    crate::attachment::Target::BeforeScenario
                }
                ScenarioHookKind::After => {
                    crate::attachment::Target::AfterScenario
                }
            }
        }
    }
    fn completed(
        &self,
        scope: &ScenarioHookScope,
        started_at: SystemTime,
        result: Result<(), String>,
        outcome: Option<crate::execution::Outcome>,
    ) {
        self.store.push(ScenarioHookRecord {
            attempt: scope.attempt.clone(),
            definition: scope.definition.clone(),
            disposition: ScenarioHookDisposition::Executed {
                invocation_id: scope.invocation_id,
                started_at,
                finished_at: SystemTime::now(),
                result,
                outcome,
            },
        });
    }
    pub(crate) async fn run_before(
        &self,
        attempt: &ScenarioHookAttempt,
        world: &mut W,
        attachments: &crate::attachment::AttachmentContext,
        timeouts: &super::Timeouts,
        execution: &crate::execution::ExecutionControl,
        policy: crate::execution::OutcomePolicy,
    ) -> super::outcomes::Summary {
        use crate::execution::OutcomeStatus as S;
        let mut summary = super::outcomes::Summary::default();
        for hook in self.before.iter().filter(|h| {
            h.applies(
                &attempt.feature,
                attempt.rule.as_deref(),
                &attempt.scenario,
            )
        }) {
            if summary.status != S::Passed {
                self.skip(
                    attempt,
                    &hook.definition,
                    if summary.status == S::Failed {
                        ScenarioHookSkipReason::PreviousFailure
                    } else {
                        ScenarioHookSkipReason::PreviousNonPassing
                    },
                );
                continue;
            }
            let scope = ScenarioHookScope {
                attempt: attempt.clone(),
                definition: hook.definition.clone(),
                invocation_id: next_id(),
                body_snapshot: None,
                result: None,
                outcome: None,
            };
            let guard = attachments.enter(self.target(&hook.definition));
            let control = execution.enter();
            let started_at = SystemTime::now();
            let callback_world = &mut *world;
            let callback_scope = &scope;
            let result = timeouts
                .invoke_callback(
                    move || match &hook.callback {
                        BeforeCallback::Legacy(callback) => callback(
                            &attempt.feature,
                            attempt.rule.as_deref(),
                            &attempt.scenario,
                            callback_world,
                        ),
                        BeforeCallback::Scoped(callback) => {
                            callback(callback_scope, callback_world)
                        }
                    },
                    hook.timeout,
                )
                .await
                .map_err(|e| super::step_hooks::panic_message(e.as_ref()));
            let completion = control.finish();
            drop(guard);
            let status = super::outcomes::status(&result, completion);
            summary.status = status;
            self.completed(
                &scope,
                started_at,
                result.clone(),
                super::outcomes::metadata(policy, status),
            );
            if let Err(message) = result {
                summary.failures.push(format!(
                    "Before hook `{}` failed: {message}",
                    hook.definition.name
                ));
            }
        }
        summary
    }
    pub(crate) async fn run_after(
        &self,
        attempt: &ScenarioHookAttempt,
        body: &ScenarioFinished,
        mut world: Option<&mut W>,
        attachments: &crate::attachment::AttachmentContext,
        timeouts: &super::Timeouts,
        execution: &crate::execution::ExecutionControl,
        policy: crate::execution::OutcomePolicy,
        body_outcome: crate::execution::Outcome,
    ) -> super::outcomes::Summary {
        use crate::execution::OutcomeStatus as S;
        let mut prior = ScenarioHookResult::from_body(
            body,
            self.store.failures(attempt.scenario_id),
            attempt.retries_remaining,
        );
        let mut aggregate = body_outcome.status;
        if policy != crate::execution::OutcomePolicy::Legacy {
            prior.apply_outcome(body_outcome, attempt.retries_remaining);
        }
        let empty_compat = self.empty == EmptyScenarioHooks::CucumberJs13_3
            && !attempt.has_steps;
        let mut summary = super::outcomes::Summary::default();
        for hook in self.after.iter().rev().filter(|h| {
            h.applies(
                &attempt.feature,
                attempt.rule.as_deref(),
                &attempt.scenario,
            )
        }) {
            if empty_compat && aggregate != S::Passed {
                self.skip(
                    attempt,
                    &hook.definition,
                    if aggregate == S::Failed {
                        ScenarioHookSkipReason::PreviousFailure
                    } else {
                        ScenarioHookSkipReason::PreviousNonPassing
                    },
                );
                continue;
            }
            let scope = ScenarioHookScope {
                attempt: attempt.clone(),
                definition: hook.definition.clone(),
                invocation_id: next_id(),
                body_snapshot: Some(body.clone()),
                result: (!empty_compat).then(|| prior.clone()),
                outcome: if empty_compat {
                    None
                } else {
                    super::outcomes::metadata(policy, aggregate)
                },
            };
            let guard = attachments.enter(self.target(&hook.definition));
            let control = execution.enter();
            let started_at = SystemTime::now();
            let callback_world = world.as_deref_mut();
            let callback_scope = &scope;
            let result = timeouts
                .invoke_callback(
                    move || match &hook.callback {
                        AfterCallback::Legacy(callback) => callback(
                            &attempt.feature,
                            attempt.rule.as_deref(),
                            &attempt.scenario,
                            body,
                            callback_world,
                        ),
                        AfterCallback::Scoped(callback) => {
                            callback(callback_scope, callback_world)
                        }
                    },
                    hook.timeout,
                )
                .await
                .map_err(|e| super::step_hooks::panic_message(e.as_ref()));
            let completion = control.finish();
            drop(guard);
            let status = super::outcomes::status(&result, completion);
            summary.status = summary.status.max(status);
            aggregate = aggregate.max(status);
            self.completed(
                &scope,
                started_at,
                result.clone(),
                super::outcomes::metadata(policy, status),
            );
            if let Err(message) = result {
                summary
                    .failures
                    .push(format!("`{}`: {message}", hook.definition.name));
                prior.failed(
                    ScenarioHookFailure {
                        definition: hook.definition.clone(),
                        message,
                    },
                    attempt.retries_remaining,
                );
            }
            if policy != crate::execution::OutcomePolicy::Legacy {
                prior.apply_outcome(
                    policy.outcome(aggregate),
                    attempt.retries_remaining,
                );
            }
        }
        if !summary.failures.is_empty() {
            summary.failures = vec![format!(
                "After hooks failed: {}",
                summary.failures.join("; ")
            )];
        }
        summary
    }
}

impl<W> Hooks<W> {
    pub(crate) fn has_before(&self) -> bool {
        !self.before.is_empty()
    }
    pub(crate) fn has_after(&self) -> bool {
        !self.after.is_empty()
    }
    pub(crate) fn clear_before(&mut self) {
        self.before.clear();
    }
    pub(crate) fn clear_after(&mut self) {
        self.after.clear();
    }
}
