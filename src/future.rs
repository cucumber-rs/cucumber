//! Aiding [`Future`]s definitions.

use std::{future::Future, pin::Pin, task};

/// Wakes the current task and returns [`task::Poll::Pending`] once.
///
/// This function is useful when we want to cooperatively give time to a task
/// scheduler. It's generally a good idea to yield inside loops, because this
/// way we make sure long-running tasks don’t prevent other tasks from running.
pub(crate) const fn yield_now() -> YieldNow {
    YieldNow(false)
}

/// [`Future`] returned by the [`yield_now()`] function.
#[derive(Clone, Copy, Debug)]
pub(crate) struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        if self.0 {
            task::Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            task::Poll::Pending
        }
    }
}
