// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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

use crate::{cli, event, parser, writer, Event, World, Writer};

/// Wrapper for passing events to multiple terminating [`Writer`]s
/// simultaneously.
///
/// # Blanket implementations
///
/// [`ArbitraryWriter`] and [`StatsWriter`] are implemented only in case both
/// `left` and `right` [`Writer`]s implement them. In case one of them doesn't
/// implement the required traits, use
/// [`WriterExt::discard_arbitrary_writes()`][1] and
/// [`WriterExt::discard_stats_writes()`][2] methods to provide the one with
/// no-op implementations.
///
/// [`ArbitraryWriter`]: writer::Arbitrary
/// [`StatsWriter`]: writer::Stats
/// [1]: crate::WriterExt::discard_arbitrary_writes
/// [2]: crate::WriterExt::discard_stats_writes
#[derive(Clone, Copy, Debug)]
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

#[warn(clippy::missing_trait_methods)]
#[async_trait(?Send)]
impl<'val, W, L, R, Val> writer::Arbitrary<'val, W, Val> for Tee<L, R>
where
    W: World,
    L: writer::Arbitrary<'val, W, Val>,
    R: writer::Arbitrary<'val, W, Val>,
    Val: Clone + 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        future::join(self.left.write(val.clone()), self.right.write(val)).await;
    }
}

impl<W, L, R> writer::Stats<W> for Tee<L, R>
where
    L: writer::Stats<W>,
    R: writer::Stats<W>,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        // Either one of them is zero, or both numbers are the same.
        cmp::max(self.left.passed_steps(), self.right.passed_steps())
    }

    fn skipped_steps(&self) -> usize {
        // Either one of them is zero, or both numbers are the same.
        cmp::max(self.left.skipped_steps(), self.right.skipped_steps())
    }

    fn failed_steps(&self) -> usize {
        // Either one of them is zero, or both numbers are the same.
        cmp::max(self.left.failed_steps(), self.right.failed_steps())
    }

    fn retried_steps(&self) -> usize {
        // Either one of them is zero, or both numbers are the same.
        cmp::max(self.left.retried_steps(), self.right.retried_steps())
    }

    fn parsing_errors(&self) -> usize {
        // Either one of them is zero, or both numbers are the same.
        cmp::max(self.left.parsing_errors(), self.right.parsing_errors())
    }

    fn hook_errors(&self) -> usize {
        // Either one of them is zero, or both numbers are the same.
        cmp::max(self.left.hook_errors(), self.right.hook_errors())
    }
}

#[warn(clippy::missing_trait_methods)]
impl<L, R> writer::Normalized for Tee<L, R>
where
    L: writer::Normalized,
    R: writer::Normalized,
{
}

#[warn(clippy::missing_trait_methods)]
impl<L, R> writer::NonTransforming for Tee<L, R>
where
    L: writer::NonTransforming,
    R: writer::NonTransforming,
{
}
