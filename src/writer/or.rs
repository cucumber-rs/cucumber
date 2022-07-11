// Copyright (c) 2018-2022  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Passing events to one of two [`Writer`]s based on a predicate.

use async_trait::async_trait;
use futures::future;

use crate::{cli, event, parser, writer, Event, World, Writer};

/// Wrapper for passing events to one of two [`Writer`]s based on a predicate.
#[derive(Clone, Copy, Debug)]
pub struct Or<L, R, F> {
    /// Left [`Writer`].
    pub left: L,

    /// Right [`Writer`].
    pub right: R,

    /// Indicates, which [`Writer`] should be used. `left` is used on [`true`]
    /// and `right` on [`false`].
    predicate: F,
}

/// Wrapper for [`Or`] [`Writer`], that implements [`writer::Arbitrary`],
/// writing into both [`Writer`]s.
#[derive(Clone, Copy, Debug)]
pub struct ArbitraryWriteIntoBoth<T>(pub T);

impl<L, R, F> Or<L, R, F> {
    /// Creates a new [`Or`] [`Writer`], which passes events to the `left`
    /// and `right` [`Writer`]s based on a `predicate`.
    ///
    /// In case `predicate` returns [`true`], `left` [`Writer`] is used and
    /// `right` [`Writer`] is used on [`false`].
    #[must_use]
    pub const fn new(left: L, right: R, predicate: F) -> Self {
        Self {
            left,
            right,
            predicate,
        }
    }

    /// Wraps this [`Writer`] into a type, that implements
    /// [`writer::Arbitrary`] by writing into both [`Writer`]s.
    #[must_use]
    pub const fn arbitrary_write_into_both(
        self,
    ) -> ArbitraryWriteIntoBoth<Self> {
        ArbitraryWriteIntoBoth(self)
    }
}

#[async_trait(?Send)]
impl<W, L, R, F> Writer<W> for Or<L, R, F>
where
    W: World,
    L: Writer<W>,
    R: Writer<W>,
    F: FnMut(
        &parser::Result<Event<event::Cucumber<W>>>,
        &cli::Compose<L::Cli, R::Cli>,
    ) -> bool,
{
    type Cli = cli::Compose<L::Cli, R::Cli>;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        if (self.predicate)(&ev, cli) {
            self.left.handle_event(ev, &cli.left).await;
        } else {
            self.right.handle_event(ev, &cli.right).await;
        }
    }
}

#[async_trait(?Send)]
impl<W, L, R, F> Writer<W> for ArbitraryWriteIntoBoth<Or<L, R, F>>
where
    W: World,
    L: Writer<W>,
    R: Writer<W>,
    F: FnMut(
        &parser::Result<Event<event::Cucumber<W>>>,
        &cli::Compose<L::Cli, R::Cli>,
    ) -> bool,
{
    type Cli = cli::Compose<L::Cli, R::Cli>;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.0.handle_event(ev, cli).await;
    }
}

impl<W, L, R, F> writer::Failure<W> for Or<L, R, F>
where
    L: writer::Failure<W>,
    R: writer::Failure<W>,
    F: FnMut(
        &parser::Result<Event<event::Cucumber<W>>>,
        &cli::Compose<L::Cli, R::Cli>,
    ) -> bool,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        self.left.failed_steps() + self.right.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.left.parsing_errors() + self.right.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.left.hook_errors() + self.right.hook_errors()
    }
}

impl<W, L, R, F> writer::Failure<W> for ArbitraryWriteIntoBoth<Or<L, R, F>>
where
    L: writer::Failure<W>,
    R: writer::Failure<W>,
    F: FnMut(
        &parser::Result<Event<event::Cucumber<W>>>,
        &cli::Compose<L::Cli, R::Cli>,
    ) -> bool,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        self.0.left.failed_steps() + self.0.right.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.0.left.parsing_errors() + self.0.right.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.0.left.hook_errors() + self.0.right.hook_errors()
    }
}

impl<L, R, F> writer::Normalized for Or<L, R, F>
where
    L: writer::Normalized,
    R: writer::Normalized,
{
}

impl<L, R, F> writer::Normalized for ArbitraryWriteIntoBoth<Or<L, R, F>>
where
    L: writer::Normalized,
    R: writer::Normalized,
{
}

impl<L, R, F> writer::NonTransforming for ArbitraryWriteIntoBoth<Or<L, R, F>>
where
    L: writer::NonTransforming,
    R: writer::NonTransforming,
{
}

#[async_trait(?Send)]
impl<'val, W, L, R, Val, F> writer::Arbitrary<'val, W, Val>
    for ArbitraryWriteIntoBoth<Or<L, R, F>>
where
    W: World,
    L: writer::Arbitrary<'val, W, Val>,
    R: writer::Arbitrary<'val, W, Val>,
    Val: Clone + 'val,
    F: FnMut(
        &parser::Result<Event<event::Cucumber<W>>>,
        &cli::Compose<L::Cli, R::Cli>,
    ) -> bool,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        future::join(self.0.left.write(val.clone()), self.0.right.write(val))
            .await;
    }
}
