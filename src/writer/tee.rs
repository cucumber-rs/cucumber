// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Passing events to multiple terminating [`Writer`]s simultaneously.

use std::cmp;

use async_trait::async_trait;
use futures::future;

use crate::{
    cli, event, parser, ArbitraryWriter, Event, FailureWriter, World, Writer,
};

/// Wrapper for passing events to multiple terminating [`Writer`]s
/// simultaneously.
///
/// # Blanket implementations
///
/// [`ArbitraryWriter`] and [`FailureWriter`] are implemented only in case both
/// `left` and `right` [`Writer`] implements them. In case one of them doesn't
/// implement required traits, use [`WriterExt::discard_arbitrary()`][1] and
/// [`WriterExt::discard_failure()`][2].
///
/// [1]: crate::WriterExt::discard_arbitrary()
/// [2]: crate::WriterExt::discard_failure()
#[derive(Clone, Debug)]
pub struct Tee<L, R> {
    /// Left [`Writer`].
    left: L,

    /// Right [`Writer`].
    right: R,
}

impl<L, R> Tee<L, R> {
    /// Creates a new [`Tee`] [`Writer`], which passes events both to the `left`
    /// and `right` [`Writer`]s simultaneously.
    #[must_use]
    pub const fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

#[async_trait(?Send)]
impl<W, L, R> Writer<W> for Tee<L, R>
where
    W: World,
    L: Writer<W>,
    R: Writer<W>,
{
    type Cli = cli::Compose<L::Cli, R::Cli>;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        future::join(
            self.left.handle_event(ev.clone(), &cli.left),
            self.right.handle_event(ev, &cli.right),
        )
        .await;
    }
}

#[async_trait(?Send)]
impl<'val, W, L, R, Val> ArbitraryWriter<'val, W, Val> for Tee<L, R>
where
    W: World,
    L: ArbitraryWriter<'val, W, Val>,
    R: ArbitraryWriter<'val, W, Val>,
    Val: Clone + 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        future::join(self.left.write(val.clone()), self.right.write(val)).await;
    }
}

impl<W, L, R> FailureWriter<W> for Tee<L, R>
where
    L: FailureWriter<W>,
    R: FailureWriter<W>,
    R: Writer<W>,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        cmp::max(self.left.failed_steps(), self.right.failed_steps())
    }

    fn parsing_errors(&self) -> usize {
        cmp::max(self.left.parsing_errors(), self.right.parsing_errors())
    }

    fn hook_errors(&self) -> usize {
        cmp::max(self.left.hook_errors(), self.right.hook_errors())
    }
}
