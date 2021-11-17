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
/// [`ArbitraryWriter`] and [`FailureWriter`] are implemented only in case the
/// `left` [`Writer`] implements them. This is done to achieve a balance between
/// being able to [`tee()`] 3 or more writers, while imposing
/// minimal trait bounds.
///
/// Unfortunately, for now it's impossible to pass [`ArbitraryWriter`]s `Val`
/// additionally to the `right` [`Writer`] in case it implements
/// [`ArbitraryWriter`].
///
/// [`tee()`]: crate::WriterExt::tee()
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
    R: Writer<W>,
    Val: 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.left.write(val).await;
    }
}

impl<W, L, R> FailureWriter<W> for Tee<L, R>
where
    L: FailureWriter<W>,
    R: Writer<W>,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        self.left.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.left.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.left.hook_errors()
    }
}
