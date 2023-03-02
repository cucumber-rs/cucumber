//! Aiding [`Future`]s definitions.

use std::{future::Future, pin::Pin, task};

use futures::{
    future::{Either, FusedFuture, Then},
    FutureExt as _,
};
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

/// Return type of a [`FutureExt::then_yield()`] method.
type ThenYield<F, O> = Then<F, YieldThenReturn<O>, fn(O) -> YieldThenReturn<O>>;

/// Extensions of a [`Future`], used inside this crate.
pub(crate) trait FutureExt: Future + Sized {
    /// Yields after this [`Future`] is resolved allowing other [`Future`]s
    /// making progress.
    fn then_yield(self) -> ThenYield<Self, Self::Output> {
        self.then(YieldThenReturn::new)
    }
}

impl<T: Future> FutureExt for T {}

/// [`Future`] returning a [`task::Poll::Pending`] once, before returning a
/// contained value.
#[derive(Debug)]
#[pin_project]
pub(crate) struct YieldThenReturn<V> {
    /// Value to be returned.
    value: Option<V>,

    /// [`YieldNow`] [`Future`].
    r#yield: YieldNow,
}

impl<V> YieldThenReturn<V> {
    /// Creates a new [`YieldThenReturn`] [`Future`].
    const fn new(v: V) -> Self {
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
        task::ready!(this.r#yield.poll_unpin(cx));
        this.value
            .take()
            .map_or(task::Poll::Pending, task::Poll::Ready)
    }
}

/// [`select`] that always [`poll()`]s the `biased` [`Future`] first, and only
/// if it returns [`task::Poll::Pending`] tries to [`poll()`] the `regular` one.
///
/// Implementation is exactly the same, as [`select`] at the moment, but
/// documentation has no guarantees about this behaviour, so can be changed.
///
/// [`poll()`]: Future::poll
/// [`select`]: futures::future::select
pub(crate) const fn select_with_biased_first<A, B>(
    biased: A,
    regular: B,
) -> SelectWithBiasedFirst<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    SelectWithBiasedFirst {
        inner: Some((biased, regular)),
    }
}

/// [`Future`] returned by a [`select_with_biased_first()`] function.
pub(crate) struct SelectWithBiasedFirst<A, B> {
    /// Inner [`Future`]s.
    inner: Option<(A, B)>,
}

impl<A, B> Future for SelectWithBiasedFirst<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    type Output = Either<(A::Output, B), (B::Output, A)>;

    #[allow(clippy::expect_used)]
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let (mut a, mut b) = self
            .inner
            .take()
            .expect("cannot poll `SelectWithBiasedFirst` twice");

        if let task::Poll::Ready(val) = a.poll_unpin(cx) {
            return task::Poll::Ready(Either::Left((val, b)));
        }

        if let task::Poll::Ready(val) = b.poll_unpin(cx) {
            return task::Poll::Ready(Either::Right((val, a)));
        }

        self.inner = Some((a, b));
        task::Poll::Pending
    }
}

impl<A, B> FusedFuture for SelectWithBiasedFirst<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    fn is_terminated(&self) -> bool {
        self.inner.is_none()
    }
}
