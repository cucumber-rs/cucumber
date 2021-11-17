// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Wrapper for adding an empty [`FailureWriter`] implementation.

use async_trait::async_trait;
use derive_more::{Deref, DerefMut, From};

use crate::{
    event::Cucumber, ArbitraryWriter, Event, FailureWriter, World, Writer,
};

/// Wrapper for adding an empty [`FailureWriter`] implementation.
///
/// Can be useful for one of the [`Writer`]s in [`writer::Tee`].
///
/// [`writer::Tee`]: crate::writer::Tee
#[derive(Clone, Debug, Deref, DerefMut, From)]
pub struct FailureDiscard<Wr>(Wr);

#[async_trait(?Send)]
impl<W: World, Wr: Writer<W>> Writer<W> for FailureDiscard<Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        ev: crate::parser::Result<Event<Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.0.handle_event(ev, cli).await;
    }
}

#[async_trait(?Send)]
impl<'val, W: World, Val: 'val, Wr: ArbitraryWriter<'val, W, Val>>
    ArbitraryWriter<'val, W, Val> for FailureDiscard<Wr>
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.0.write(val).await;
    }
}

impl<W: World, Wr: Writer<W>> FailureWriter<W> for FailureDiscard<Wr> {
    fn failed_steps(&self) -> usize {
        0
    }

    fn parsing_errors(&self) -> usize {
        0
    }

    fn hook_errors(&self) -> usize {
        0
    }
}

impl<Wr> FailureDiscard<Wr> {
    /// Creates a new [`FailureDiscard`].
    #[must_use]
    pub const fn new(writer: Wr) -> Self {
        Self(writer)
    }
}
