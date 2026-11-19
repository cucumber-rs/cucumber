//! Typed evidence bound to runner-owned step and scenario-hook scopes.
//!
//! Use a fresh shared store per run. The runner supplies a context through
//! World::new_with_context. A handle captured from that context remains bound
//! to one scope and rejects writes after that scope closes; it never retargets.

use std::{
    collections::HashMap,
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::SystemTime,
};

/// Caller-owned text or binary evidence with media type and optional name.
#[derive(Clone)]
pub struct Attachment {
    /// Caller-provided payload in its original representation.
    pub(crate) body: Body,
    /// Unmodified media type supplied with the payload.
    pub(crate) media_type: String,
    /// Optional caller-provided file name.
    pub(crate) file_name: Option<String>,
}
/// Original text or binary payload, before writer-specific encoding.
#[derive(Clone)]
pub(crate) enum Body {
    /// Text retained without encoding conversion.
    Text(String),
    /// Bytes retained without encoding conversion.
    Bytes(Vec<u8>),
}
// Keep evidence visible in diagnostics even without a reporting writer.
impl fmt::Debug for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attachment")
            .field("body", &self.body)
            .field("media_type", &self.media_type)
            .field("file_name", &self.file_name)
            .finish()
    }
}
impl fmt::Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(body) => f.debug_tuple("Text").field(body).finish(),
            Self::Bytes(body) => f.debug_tuple("Bytes").field(body).finish(),
        }
    }
}
impl Attachment {
    #[must_use]
    /// Creates text evidence without changing its content or media type.
    pub fn text(
        body: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Self {
        Self {
            body: Body::Text(body.into()),
            media_type: media_type.into(),
            file_name: None,
        }
    }
    #[must_use]
    /// Creates binary evidence; the reporting writer chooses its wire encoding.
    pub fn bytes(
        body: impl Into<Vec<u8>>,
        media_type: impl Into<String>,
    ) -> Self {
        Self {
            body: Body::Bytes(body.into()),
            media_type: media_type.into(),
            file_name: None,
        }
    }
    #[must_use]
    /// Retains the caller-provided suggested file name exactly.
    pub fn with_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }
}

/// Failure to acquire an enabled scope or append to a closed one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AttachmentError {
    /// The runner has no attachment store configured.
    Disabled,
    /// There is currently no executing step or scenario-hook scope.
    NoActiveScope,
    /// The handle's original scope has finished or was cancelled.
    ClosedScope,
}
impl fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Disabled => "attachments are disabled",
            Self::NoActiveScope => "no active attachment scope",
            Self::ClosedScope => "attachment scope is closed",
        })
    }
}
impl std::error::Error for AttachmentError {}

/// Identity of one scenario attempt in its source feature and rule.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct AttemptKey {
    /// Source feature path.
    path: Option<PathBuf>,
    /// Parent rule with its source identity preserved.
    rule: Option<gherkin::Rule>,
    /// Executed scenario identity.
    scenario: gherkin::Scenario,
    /// Zero-based retry attempt.
    attempt: usize,
}
/// Execution segment owning an attachment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Target {
    /// Legacy before-scenario hook.
    BeforeScenario,
    /// Zero-based step ordinal.
    Step(usize),
    /// Legacy after-scenario hook.
    AfterScenario,
    /// Registered scenario hook identifier.
    ScenarioHook(u64),
}
/// Payload and execution identity captured at attachment time.
#[derive(Clone)]
pub(crate) struct Record {
    /// Owning execution segment.
    pub target: Target,
    /// Original payload and its media metadata.
    pub attachment: Attachment,
    /// Time the scope accepted this payload.
    pub at: SystemTime,
}

impl fmt::Debug for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Record")
            .field("target", &self.target)
            .field("attachment", &self.attachment)
            .field("at", &self.at)
            .finish()
    }
}

/// Shared evidence queue for one runner and its Messages writer.
///
/// Clones share queued records. Use a fresh store for every independent run.
/// Messages consumes each attempt's records once when its execution completes.
/// Other writers may ignore this explicitly configured capability.
#[derive(Clone, Debug, Default)]
pub struct AttachmentStore(Arc<Mutex<HashMap<AttemptKey, Vec<Record>>>>);
impl AttachmentStore {
    #[must_use]
    /// Creates an empty store for one independent run.
    pub fn new() -> Self {
        Self::default()
    }
    /// Whether reporting has drained every queued attempt.
    #[cfg(feature = "output-messages")]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.lock().expect("attachment store poisoned").is_empty()
    }
    /// Creates isolated scope bookkeeping for one scenario attempt.
    pub(crate) fn context(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        attempt: usize,
    ) -> AttachmentContext {
        AttachmentContext(Some(Arc::new(ContextState {
            store: self.clone(),
            key: AttemptKey {
                path: feature.path.clone(),
                rule: crate::feature::rule_identity(rule),
                scenario: scenario.clone(),
                attempt,
            },
            active: Mutex::new(None),
            next_step: Mutex::new(0),
        })))
    }
    /// Removes and returns one completed attempt exactly once.
    #[cfg(feature = "output-messages")]
    pub(crate) fn take(
        &self,
        path: &str,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        attempt: usize,
    ) -> Vec<Record> {
        self.0
            .lock()
            .expect("attachment store poisoned")
            .remove(&AttemptKey {
                path: Some(PathBuf::from(path)),
                rule: crate::feature::rule_identity(rule),
                scenario: scenario.clone(),
                attempt,
            })
            .unwrap_or_default()
    }
}

/// Per-attempt access to the runner's currently executing attachment scope.
///
/// Store this in an attachment-aware World. Acquire a fresh handle inside each
/// step or scenario-hook callback. A retained context can acquire a new scope;
/// a retained handle always remains bound to its original scope.
#[derive(Clone, Debug, Default)]
pub struct AttachmentContext(Option<Arc<ContextState>>);
/// Mutable scope bookkeeping shared by a single scenario attempt.
#[derive(Debug)]
struct ContextState {
    /// Queue shared with the reporting writer.
    store: AttachmentStore,
    /// Identity of the owning attempt.
    key: AttemptKey,
    /// Currently open execution scope, when one exists.
    active: Mutex<Option<Arc<Mutex<ScopeState>>>>,
    /// Ordinal assigned to the next native step.
    next_step: Mutex<usize>,
}
/// Scope identity and acceptance flag synchronized with attachment writes.
#[derive(Debug)]
struct ScopeState {
    /// Whether the runner still permits writes to this scope.
    open: bool,
    /// Queue shared with the reporting writer.
    store: AttachmentStore,
    /// Identity of the owning attempt.
    key: AttemptKey,
    /// Owning execution segment.
    target: Target,
}
impl AttachmentContext {
    /// Obtains a handle for the current step or scenario-hook invocation.
    pub fn scope(&self) -> Result<AttachmentHandle, AttachmentError> {
        let context = self.0.as_ref().ok_or(AttachmentError::Disabled)?;
        let active = context
            .active
            .lock()
            .expect("attachment context poisoned")
            .clone()
            .ok_or(AttachmentError::NoActiveScope)?;
        if !active.lock().expect("attachment scope poisoned").open {
            return Err(AttachmentError::ClosedScope);
        }
        Ok(AttachmentHandle(active))
    }
    /// Opens the next native step scope.
    pub(crate) fn enter_step(&self) -> ScopeGuard {
        let Some(context) = &self.0 else { return ScopeGuard::default() };
        let ordinal = {
            let mut next =
                context.next_step.lock().expect("attachment ordinal poisoned");
            let ordinal = *next;
            *next += 1;
            ordinal
        };
        self.enter(Target::Step(ordinal))
    }
    /// Opens one execution segment, rejecting overlapping scopes.
    pub(crate) fn enter(&self, target: Target) -> ScopeGuard {
        let Some(context) = &self.0 else { return ScopeGuard::default() };
        let mut active =
            context.active.lock().expect("attachment context poisoned");
        assert!(
            active.is_none(),
            "runner entered overlapping attachment scopes"
        );
        let scope = Arc::new(Mutex::new(ScopeState {
            open: true,
            store: context.store.clone(),
            key: context.key.clone(),
            target,
        }));
        *active = Some(scope.clone());
        ScopeGuard { context: Some(context.clone()), scope: Some(scope) }
    }
}

/// Cloneable evidence sink permanently bound to one execution scope.
///
/// Clones share the same lifetime boundary. Attaching and closing use
/// the same lock, so a write cannot be accepted after closure, even from a task
/// that retained a handle while another step or retry begins.
#[derive(Clone, Debug)]
pub struct AttachmentHandle(Arc<Mutex<ScopeState>>);
impl AttachmentHandle {
    /// Appends evidence to this handle's original scope while it is open.
    ///
    /// Returns [`AttachmentError::ClosedScope`] after completion or
    /// cancellation.
    pub fn attach(
        &self,
        attachment: Attachment,
    ) -> Result<(), AttachmentError> {
        let scope = self.0.lock().expect("attachment scope poisoned");
        if !scope.open {
            return Err(AttachmentError::ClosedScope);
        }
        let record =
            Record { target: scope.target, attachment, at: SystemTime::now() };
        scope
            .store
            .0
            .lock()
            .expect("attachment store poisoned")
            .entry(scope.key.clone())
            .or_default()
            .push(record);
        Ok(())
    }
}

/// Closes scopes on normal completion, unwind, or future cancellation.
#[derive(Debug, Default)]
pub(crate) struct ScopeGuard {
    /// Attempt whose active scope must be cleared on closure.
    context: Option<Arc<ContextState>>,
    /// Exact scope closed by this guard, including retained handles.
    scope: Option<Arc<Mutex<ScopeState>>>,
}
impl ScopeGuard {
    /// Invalidates retained handles after a bounded invocation fails while
    /// preserving the native step target for its remaining cleanup callbacks.
    pub(crate) fn renew(&mut self) {
        let Some(context) = self.context.clone() else { return };
        let target = self
            .scope
            .as_ref()
            .expect("enabled scope")
            .lock()
            .expect("attachment scope poisoned")
            .target;
        *self = Self::default();
        *self = AttachmentContext(Some(context)).enter(target);
    }
}
impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let Some(scope) = &self.scope else { return };
        scope.lock().expect("attachment scope poisoned").open = false;
        if let Some(context) = &self.context {
            let mut active =
                context.active.lock().expect("attachment context poisoned");
            if active
                .as_ref()
                .is_some_and(|candidate| Arc::ptr_eq(candidate, scope))
            {
                *active = None;
            }
        }
    }
}
