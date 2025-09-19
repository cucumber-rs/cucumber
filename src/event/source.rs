//! Source wrapper for gherkin types.

use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use derive_more::with_trait::{AsRef, Debug, Deref, Display, From, Into};
use ref_cast::RefCast;

/// Wrappers around a [`gherkin`] type ([`gherkin::Feature`],
/// [`gherkin::Scenario`], etc.), providing cheap [`Clone`], [`Hash`] and
/// [`PartialEq`] implementations for using it extensively in [`Event`]s.
///
/// [`Event`]: super::Event
#[derive(AsRef, Debug, Deref, Display, From, Into, RefCast)]
#[as_ref(forward)]
#[debug("{:?}", **_0)]
#[debug(bound(T: std::fmt::Debug))]
#[deref(forward)]
#[repr(transparent)]
pub struct Source<T: ?Sized>(Arc<T>);

impl<T> Source<T> {
    /// Wraps the provided `value` into a new [`Source`].
    #[must_use]
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}

// Manual implementation is required to omit the redundant `T: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<T> Clone for Source<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

// Manual implementation is required to omit the redundant `T: Eq` trait bound
// imposed by `#[derive(Eq)]`.
impl<T: ?Sized> Eq for Source<T> {}

impl<T: ?Sized> PartialEq for Source<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> Hash for Source<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}