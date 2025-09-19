//! Core Event struct and implementations.

#[cfg(feature = "timestamps")]
use std::time::SystemTime;

use derive_more::with_trait::{AsRef, Debug, Deref, DerefMut};

/// Alias for a [`catch_unwind()`] error.
///
/// [`catch_unwind()`]: std::panic::catch_unwind()
pub type Info = std::sync::Arc<dyn std::any::Any + Send + 'static>;

/// Arbitrary event, optionally paired with additional metadata.
///
/// Any metadata is added by enabling the correspondent library feature:
/// - `timestamps`: adds time of when this [`Event`] has happened.
#[derive(AsRef, Clone, Copy, Debug, Deref, DerefMut)]
#[non_exhaustive]
pub struct Event<T: ?Sized> {
    /// [`SystemTime`] when this [`Event`] has happened.
    #[cfg(feature = "timestamps")]
    pub at: SystemTime,

    /// Actual value of this [`Event`].
    #[as_ref]
    #[deref]
    #[deref_mut]
    pub value: T,
}

impl<T> Event<T> {
    /// Creates a new [`Event`] out of the given `value`.
    #[cfg_attr(
        not(feature = "timestamps"),
        expect(clippy::missing_const_for_fn, reason = "API compliance")
    )]
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            #[cfg(feature = "timestamps")]
            at: SystemTime::now(),
            value,
        }
    }

    /// Unwraps the inner [`Event::value`] loosing all the attached metadata.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Splits this [`Event`] to the inner [`Event::value`] and its detached
    /// metadata.
    #[must_use]
    pub fn split(self) -> (T, Metadata) {
        self.replace(())
    }

    /// Replaces the inner [`Event::value`] with the given one, dropping the old
    /// one in place.
    #[must_use]
    pub fn insert<V>(self, value: V) -> Event<V> {
        self.replace(value).1
    }

    /// Maps the inner [`Event::value`] with the given function.
    #[must_use]
    pub fn map<V>(self, f: impl FnOnce(T) -> V) -> Event<V> {
        let (val, meta) = self.split();
        meta.insert(f(val))
    }

    /// Replaces the inner [`Event::value`] with the given one, returning the
    /// old one along.
    #[must_use]
    pub fn replace<V>(self, value: V) -> (T, Event<V>) {
        let event = Event {
            #[cfg(feature = "timestamps")]
            at: self.at,
            value,
        };
        (self.value, event)
    }
}

/// Shortcut for a detached metadata of an arbitrary [`Event`].
pub type Metadata = Event<()>;

impl Metadata {
    /// Wraps the given `value` with this [`Event`] metadata.
    #[must_use]
    pub fn wrap<V>(self, value: V) -> Event<V> {
        self.replace(value).1
    }
}