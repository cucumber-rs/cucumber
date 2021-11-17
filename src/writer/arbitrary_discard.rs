// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Wrapper for adding an empty [`ArbitraryDiscard`] implementation.

use async_trait::async_trait;
use derive_more::{Deref, DerefMut, From};

use crate::{
    event::Cucumber, writer, ArbitraryWriter, Event, FailureWriter, World,
    Writer,
};

/// Wrapper for adding an empty [`ArbitraryDiscard`] implementation.
///
/// Can be useful for one of the [`Writer`]s in [`writer::Tee`].
///
/// [`writer::Tee`]: crate::writer::Tee
#[derive(Clone, Debug, Deref, DerefMut, From)]
pub struct ArbitraryDiscard<Wr>(Wr);

#[async_trait(?Send)]
impl<W: World, Wr: Writer<W>> Writer<W> for ArbitraryDiscard<Wr> {
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
impl<'val, W: World, Val: 'val, Wr: Writer<W>> ArbitraryWriter<'val, W, Val>
    for ArbitraryDiscard<Wr>
{
    async fn write(&mut self, _: Val)
    where
        'val: 'async_trait,
    {
    }
}

impl<W: World, Wr: FailureWriter<W>> FailureWriter<W> for ArbitraryDiscard<Wr> {
    fn failed_steps(&self) -> usize {
        self.0.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.0.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.0.hook_errors()
    }
}

impl<Wr: writer::Normalized> writer::Normalized for ArbitraryDiscard<Wr> {}

impl<Wr: writer::NotTransformEvents> writer::NotTransformEvents
    for ArbitraryDiscard<Wr>
{
}

impl<Wr> ArbitraryDiscard<Wr> {
    /// Creates a new [`ArbitraryDiscard`].
    #[must_use]
    pub const fn new(writer: Wr) -> Self {
        Self(writer)
    }
}
