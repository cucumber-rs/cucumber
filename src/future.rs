//! Aiding [`Future`]s definitions.

use std::{future::Future, pin::Pin, task};

use futures::{future::Then, FutureExt as _};
use pin_project::pin_project;

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

pub(crate) trait FutureExt: Future + Sized {
    fn then_yield(
        self,
    ) -> Then<
        Self,
        YieldThenReturn<Self::Output>,
        fn(Self::Output) -> YieldThenReturn<Self::Output>,
    > {
        self.then(YieldThenReturn::new)
    }
}

impl<T: Future> FutureExt for T {}

#[derive(Debug)]
#[pin_project]
pub(crate) struct YieldThenReturn<V> {
    value: Option<V>,
    #[pin]
    r#yield: YieldNow,
}

impl<V> YieldThenReturn<V> {
    fn new(v: V) -> Self {
        Self {
            value: Some(v),
            r#yield: yield_now(),
        }
    }
}

impl<V> Future for YieldThenReturn<V> {
    type Output = V;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let this = self.project();
        task::ready!(this.r#yield.poll(cx));
        this.value
            .take()
            .map_or(task::Poll::Pending, task::Poll::Ready)
    }
}
