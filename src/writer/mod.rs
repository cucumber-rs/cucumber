// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for outputting [`Cucumber`] events.
//!
//! [`Cucumber`]: crate::event::Cucumber

pub mod basic;
pub mod normalized;
pub mod summarized;

use async_trait::async_trait;
use sealed::sealed;

use crate::{event, World};

#[doc(inline)]
pub use self::{basic::Basic, normalized::Normalized, summarized::Summarized};

/// Writer of [`Cucumber`] events to some output.
///
/// [`Cucumber`]: crate::event::Cucumber
#[async_trait(?Send)]
pub trait Writer<World> {
    /// Handles the given [`Cucumber`] event.
    ///
    /// [`Cucumber`]: crate::event::Cucumber
    async fn handle_event(&mut self, ev: event::Cucumber<World>);
}

/// [`Writer`] that also can output generic value additionally to the
/// [`Cucumber`] events.
///
/// [`Cucumber`]: crate::event::Cucumber
#[async_trait(?Send)]
pub trait Outputted<'val, World, Value: 'val>: Writer<World> {
    /// Writes `val` to some output.
    async fn write(&mut self, val: Value)
    where
        'val: 'async_trait;
}

/// Extension of [`Writer`] allowing its normalization and summarization.
#[sealed]
pub trait Ext<W: World>: Writer<W> + Sized {
    /// Wraps this [`Writer`] into a [`Normalized`] version.
    fn normalized(self) -> Normalized<W, Self>;

    /// Wraps this [`Writer`] to prints a summary at the end of an output.
    ///
    /// See [`Summarized`] for more information.
    fn summarized(self) -> Summarized<Self>;
}

#[sealed]
impl<W, T> Ext<W> for T
where
    W: World,
    T: Writer<W> + Sized,
{
    fn normalized(self) -> Normalized<W, Self> {
        Normalized::new(self)
    }

    fn summarized(self) -> Summarized<Self> {
        Summarized::new(self)
    }
}
