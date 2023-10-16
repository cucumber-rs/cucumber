// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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

use crate::{event::Cucumber, parser, writer, Event, World, Writer};

/// Wrapper providing a no-op [`ArbitraryWriter`] implementation.
///
/// Intended to be used for feeding a non-[`ArbitraryWriter`] [`Writer`] into a
/// [`writer::Tee`], as the later accepts only [`ArbitraryWriter`]s.
///
/// [`ArbitraryWriter`]: writer::Arbitrary
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Arbitrary<Wr: ?Sized>(Wr);

#[warn(clippy::missing_trait_methods)]
#[async_trait(?Send)]
impl<W: World, Wr: Writer<W> + ?Sized> Writer<W> for Arbitrary<Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.0.handle_event(ev, cli).await;
    }
}

#[warn(clippy::missing_trait_methods)]
#[async_trait(?Send)]
impl<'val, W, Val, Wr> writer::Arbitrary<'val, W, Val> for Arbitrary<Wr>
where
    Val: 'val,
    Wr: ?Sized,
    Self: Writer<W>,
{
    /// Does nothing.
    async fn write(&mut self, _: Val)
    where
        'val: 'async_trait,
    {
        // Intentionally no-op.
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr> writer::Stats<W> for Arbitrary<Wr>
where
    Wr: writer::Stats<W> + ?Sized,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        self.0.passed_steps()
    }

    fn skipped_steps(&self) -> usize {
        self.0.skipped_steps()
    }

    fn failed_steps(&self) -> usize {
        self.0.failed_steps()
    }

    fn retried_steps(&self) -> usize {
        self.0.retried_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.0.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.0.hook_errors()
    }

    fn execution_has_failed(&self) -> bool {
        self.0.execution_has_failed()
    }
}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::Normalized> writer::Normalized for Arbitrary<Wr> {}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::NonTransforming> writer::NonTransforming for Arbitrary<Wr> {}

impl<Wr> Arbitrary<Wr> {
    /// Wraps the given [`Writer`] into a [`discard::Arbitrary`] one.
    ///
    /// [`discard::Arbitrary`]: Arbitrary
    #[must_use]
    pub const fn wrap(writer: Wr) -> Self {
        Self(writer)
    }
}

/// Wrapper providing a no-op [`StatsWriter`] implementation returning only `0`.
///
/// Intended to be used for feeding a non-[`StatsWriter`] [`Writer`] into a
/// [`writer::Tee`], as the later accepts only [`StatsWriter`]s.
///
/// [`StatsWriter`]: writer::Stats
#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Stats<Wr: ?Sized>(Wr);

#[warn(clippy::missing_trait_methods)]
#[async_trait(?Send)]
impl<W: World, Wr: Writer<W> + ?Sized> Writer<W> for Stats<Wr> {
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        self.0.handle_event(ev, cli).await;
    }
}

#[warn(clippy::missing_trait_methods)]
#[async_trait(?Send)]
impl<'val, W, Val, Wr> writer::Arbitrary<'val, W, Val> for Stats<Wr>
where
    Val: 'val,
    Wr: writer::Arbitrary<'val, W, Val> + ?Sized,
    Self: Writer<W>,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.0.write(val).await;
    }
}

impl<W, Wr> writer::Stats<W> for Stats<Wr>
where
    Wr: Writer<W> + ?Sized,
    Self: Writer<W>,
{
    /// Always returns `0`.
    fn passed_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn skipped_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn failed_steps(&self) -> usize {
        0
    }

    /// Always returns `0`.
    fn retried_steps(&self) -> usize {
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

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::Normalized> writer::Normalized for Stats<Wr> {}

#[warn(clippy::missing_trait_methods)]
impl<Wr: writer::NonTransforming> writer::NonTransforming for Stats<Wr> {}

impl<Wr> Stats<Wr> {
    /// Wraps the given [`Writer`] into a [`discard::Stats`] one.
    ///
    /// [`discard::Stats`]: Stats
    #[must_use]
    pub const fn wrap(writer: Wr) -> Self {
        Self(writer)
    }
}
