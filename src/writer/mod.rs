//! Tools for outputting [`Cucumber`] events.
//!
//! [`Cucumber`]: crate::event::Cucumber

pub mod basic;
pub mod normalized;
pub mod summary;

use async_trait::async_trait;

use crate::{event, World};

#[doc(inline)]
pub use self::{basic::Basic, normalized::Normalized, summary::Summary};

/// Trait for outputting [`Cucumber`] events.
///
/// [`Cucumber`]: crate::event::Cucumber
#[async_trait(?Send)]
pub trait Writer<World> {
    /// Handles [`Cucumber`] event.
    ///
    /// [`Cucumber`]: crate::event::Cucumber
    async fn handle_event(&mut self, ev: event::Cucumber<World>);
}

/// Extension trait for [`Writer`].
pub trait WriterExt<W: World>: Writer<W> + Sized {
    /// Normalizes given [`Writer`]. See [`Normalized`] for more information.
    fn normalize(self) -> Normalized<W, Self>;

    /// Prints summary at the end. See [`Summary`] for more information.
    fn summarize(self) -> Summary<Self>;
}

impl<W, T> WriterExt<W> for T
where
    W: World,
    T: Writer<W> + Sized,
{
    fn normalize(self) -> Normalized<W, Self> {
        Normalized::new(self)
    }

    fn summarize(self) -> Summary<Self> {
        Summary::new(self)
    }
}
