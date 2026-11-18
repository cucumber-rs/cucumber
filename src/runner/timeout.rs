//! Opt-in per-invocation timeout policy and runtime-independent deadline helper.
//!
//! Legacy execution is unlimited. The JavaScript preset selects five seconds;
//! definition overrides remain local. The helper drops an expired callback
//! future rather than spawning or detaching it. Blocking polls cannot be preempted.

use std::{fmt,future::Future,sync::Arc,time::Duration};
use futures::{future::{BoxFuture,Either},pin_mut,FutureExt as _};

/// Limit for one callback/body invocation; None at a registration means inherit.
#[derive(Clone,Copy,Debug,Default,Eq,PartialEq)]
#[non_exhaustive]
pub enum TimeoutPolicy {
    /// No runner deadline, preserving legacy behavior.
    #[default]
    Disabled,
    /// Fail when this duration expires before the invocation completes.
    After(Duration),
}

/// When the synchronous callback factory runs relative to timer construction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum CallbackConstruction {
    /// Preserve native behavior: construct the timer before invoking the factory.
    #[default]
    WithinDeadline,
    /// Invoke the factory before constructing the asynchronous timer.
    BeforeDeadline,
}

/// Runtime-independent timer factory, also usable for deterministic acceptance.
///
/// Implementations return an owned timer future. They must not spawn or otherwise
/// execute the user callback; the runner continues to poll it on its own executor.
pub trait TimeoutClock: fmt::Debug+Send+Sync+'static {
    /// Creates a timer that resolves after the selected interval.
    fn sleep(&self,limit:Duration)->BoxFuture<'static,()>;
}

/// Default timer backend requiring no Tokio or other application runtime.
#[derive(Clone,Copy,Debug,Default)]
pub struct ThreadTimer;
impl TimeoutClock for ThreadTimer {
    fn sleep(&self,limit:Duration)->BoxFuture<'static,()> {
        Box::pin(futures_timer::Delay::new(limit))
    }
}

/// A completed deadline failure, retaining the actual configured limit.
#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub struct TimeoutError { limit:Duration }
impl TimeoutError {
    /// Returns the effective deadline limit used for this invocation.
    #[must_use]
    pub const fn limit(self)->Duration {self.limit}
}
impl fmt::Display for TimeoutError {
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result {
        write!(f,"callback timed out after {:?}",self.limit)
    }
}
impl std::error::Error for TimeoutError {}

/// Shared timeout defaults and timer factory for one runner.
///
/// Clone the configuration across callbacks; each invocation creates a distinct
/// timer. None overrides inherit the default, while Disabled explicitly opts out.
#[derive(Clone,Debug)]
pub struct Timeouts {default:TimeoutPolicy,clock:Arc<dyn TimeoutClock>,construction:CallbackConstruction}
impl Default for Timeouts {
    fn default()->Self {Self{default:TimeoutPolicy::Disabled,clock:Arc::new(ThreadTimer),construction:CallbackConstruction::WithinDeadline}}
}
impl Timeouts {
    /// Selects the pinned Cucumber.js five-second default explicitly.
    #[must_use]
    pub fn cucumber_js()->Self {Self::default().with_default(TimeoutPolicy::After(Duration::from_secs(5))).with_callback_construction(CallbackConstruction::BeforeDeadline)}
    /// Changes the inherited callback/body timeout without changing the clock.
    #[must_use]
    pub fn with_default(mut self,policy:TimeoutPolicy)->Self {self.default=policy;self}
    /// Supplies a timer implementation without coupling callbacks to a runtime.
    #[must_use]
    pub fn with_clock(mut self,clock:Arc<dyn TimeoutClock>)->Self {self.clock=clock;self}
    /// Selects callback-factory timing independently of timeout durations.
    /// Generated argument preparation is eager only for registrations explicitly
    /// marked `prepare_before_timeout`; arbitrary future internals stay lazy.
    #[must_use]
    pub fn with_callback_construction(mut self, policy:CallbackConstruction)->Self {self.construction=policy;self}
    /// Returns the selected callback-factory boundary.
    #[must_use]
    pub const fn callback_construction(&self)->CallbackConstruction {self.construction}
    /// Invokes the actual callback factory, catching both construction and polling.
    pub(crate) async fn invoke_callback<Make,F>(&self,factory:Make,override_policy:Option<TimeoutPolicy>)
        ->Result<(),Box<dyn std::any::Any+Send>>
    where Make:FnOnce()->F,F:Future<Output=()> {
        match self.construction {
            CallbackConstruction::WithinDeadline=>self.run_catching(async move {factory().await},override_policy).await,
            CallbackConstruction::BeforeDeadline=>{
                let future=std::panic::catch_unwind(std::panic::AssertUnwindSafe(factory))?;
                self.run_catching(future,override_policy).await
            }
        }
    }
    /// Resolves a local override, leaving the shared default unchanged.
    #[must_use]
    pub const fn effective(&self,override_policy:Option<TimeoutPolicy>)->TimeoutPolicy {
        match override_policy {Some(policy)=>policy,None=>self.default}
    }
    /// Runs a callback with the existing native panic failure representation.
    pub(crate) async fn run_catching<F:Future<Output=()>>(&self,future:F,override_policy:Option<TimeoutPolicy>)
        ->Result<(),Box<dyn std::any::Any+Send>>
    {
        match self.run(std::panic::AssertUnwindSafe(future).catch_unwind(),override_policy).await {
            Ok(result)=>result,
            Err(error)=>Err(Box::new(error)),
        }
    }
    /// Polls a callback and its timer together; expiry drops the callback future.
    ///
    /// The callback is polled first when both are ready. No Send bound is imposed
    /// on it, and no background task is spawned. Callers catch callback panics at
    /// the normal lifecycle boundary and run their established cleanup sequence.
    pub(crate) async fn run<F:Future>(&self,future:F,override_policy:Option<TimeoutPolicy>)
        ->Result<F::Output,TimeoutError>
    {
        let limit=match self.effective(override_policy) {
            TimeoutPolicy::Disabled=>return Ok(future.await),
            TimeoutPolicy::After(limit)=>limit,
        };
        let timer=self.clock.sleep(limit);
        pin_mut!(future,timer);
        match crate::future::select_with_biased_first(future,timer).await {
            Either::Left((result,_timer))=>Ok(result),
            Either::Right(((),_callback))=>Err(TimeoutError{limit}),
        }
    }
}

/// Retains the legacy native failure payload while preserving timeout identity
/// until callback cancellation and attachment lifetime handling are complete.
pub(crate) fn into_panic(error: Box<dyn std::any::Any + Send>) -> Box<dyn std::any::Any + Send> {
    match error.downcast::<TimeoutError>() {
        Ok(timeout) => Box::new(timeout.to_string()),
        Err(error) => error,
    }
}
