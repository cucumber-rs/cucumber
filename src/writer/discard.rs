// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Wrappers providing no-op implementations.

use async_trait::async_trait;
use derive_more::{Deref, DerefMut};

use crate::{event::Cucumber, writer, Event, World, Writer};

/// Wrapper providing a no-op [`ArbitraryWriter`] implementation.
///
/// Intended to be used for feeding a non-[`ArbitraryWriter`] [`Writer`] into a
/// [`writer::Tee`], as the later accepts only [`ArbitraryWriter`]s.
///
/// [`ArbitraryWriter`]: writer::Arbitrary
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Arbitrary<Wr: ?Sized>(Wr);

#[async_trait(?Send)]
impl<W: World, Wr: Writer<W> + ?Sized> Writer<W> for Arbitrary<Wr> {
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
impl<'val, W, Val, Wr> writer::Arbitrary<'val, W, Val> for Arbitrary<Wr>
where
    W: World,
    Val: 'val,
    Wr: Writer<W> + ?Sized,
{
    /// Does nothing.
    #[allow(clippy::unused_async)] // false positive: #[async_trait]
    async fn write(&mut self, _: Val)
    where
        'val: 'async_trait,
    {
        // Intentionally no-op.
    }
}

impl<W: World, Wr: writer::Failure<W> + ?Sized> writer::Failure<W>
    for Arbitrary<Wr>
{
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

impl<Wr: writer::Normalized> writer::Normalized for Arbitrary<Wr> {}

impl<Wr: writer::NonTransforming> writer::NonTransforming for Arbitrary<Wr> {}

impl<Wr> Arbitrary<Wr> {
    /// Wraps the given [`Writer`] into a [`discard::Arbitrary`] one.
    ///
    /// [`discard::Arbitrary`]: crate::writer::discard::Arbitrary
    #[must_use]
    pub const fn wrap(writer: Wr) -> Self {
        Self(writer)
    }
}

/// Wrapper providing a no-op [`FailureWriter`] implementation returning only
/// `0`.
///
/// Intended to be used for feeding a non-[`FailureWriter`] [`Writer`] into a
/// [`writer::Tee`], as the later accepts only [`FailureWriter`]s.
///
/// [`FailureWriter`]: writer::Failure
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Failure<Wr: ?Sized>(Wr);

#[async_trait(?Send)]
impl<W: World, Wr: Writer<W> + ?Sized> Writer<W> for Failure<Wr> {
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
impl<'val, W, Val, Wr> writer::Arbitrary<'val, W, Val> for Failure<Wr>
where
    W: World,
    Val: 'val,
    Wr: writer::Arbitrary<'val, W, Val> + ?Sized,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.0.write(val).await;
    }
}

impl<W: World, Wr: Writer<W> + ?Sized> writer::Failure<W> for Failure<Wr> {
    /// Always returns `0`.
    fn failed_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn parsing_errors(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn hook_errors(&self) -> usize {
        0
    }
}

impl<Wr: writer::Normalized> writer::Normalized for Failure<Wr> {}

impl<Wr: writer::NonTransforming> writer::NonTransforming for Failure<Wr> {}

impl<Wr> Failure<Wr> {
    /// Wraps the given [`Writer`] into a [`discard::Failure`] one.
    ///
    /// [`discard::Failure`]: crate::writer::discard::Failure
    #[must_use]
    pub const fn wrap(writer: Wr) -> Self {
        Self(writer)
    }
}
