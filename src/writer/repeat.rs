// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Writer`]-wrapper for re-outputting events at the end.

use std::mem;

use async_trait::async_trait;
use derive_more::Deref;

use crate::{event, parser, ArbitraryWriter, FailureWriter, World, Writer};

/// Wrapper for a [`Writer`] implementation for re-outputting events at the end,
/// based on some filter.
///
/// Useful for re-outputting [skipped] or [failed] [`Step`]s.
///
/// [failed]: crate::WriterExt::append_failed
/// [skipped]: crate::WriterExt::append_skipped
/// [`Step`]: gherkin::Step
#[derive(Debug, Deref)]
pub struct Repeat<W, Wr, F = FilterEvent<W>> {
    /// Original [`Writer`].
    #[deref]
    pub writer: Wr,

    /// Predicate to decide, whether event should be re-outputted or not.
    filter: F,

    /// Buffer of events to re-output at the end.
    events: Vec<parser::Result<event::Cucumber<W>>>,
}

/// Alias for a [`fn`] used to determine whether event should be re-outputted or
/// not.
pub type FilterEvent<W> = fn(&parser::Result<event::Cucumber<W>>) -> bool;

#[async_trait(?Send)]
impl<W, Wr, F> Writer<W> for Repeat<W, Wr, F>
where
    W: World,
    Wr: Writer<W>,
    F: Fn(&parser::Result<event::Cucumber<W>>) -> bool,
{
    async fn handle_event(&mut self, ev: parser::Result<event::Cucumber<W>>) {
        if (self.filter)(&ev) {
            self.events.push(ev.clone());
        }

        let is_finished = matches!(ev, Ok(event::Cucumber::Finished));

        self.writer.handle_event(ev).await;

        if is_finished {
            for ev in mem::take(&mut self.events) {
                self.writer.handle_event(ev).await;
            }
        }
    }
}

#[async_trait(?Send)]
impl<'val, W, Wr, Val, F> ArbitraryWriter<'val, W, Val> for Repeat<W, Wr, F>
where
    W: World,
    Wr: ArbitraryWriter<'val, W, Val>,
    Val: 'val,
    F: Fn(&parser::Result<event::Cucumber<W>>) -> bool,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.writer.write(val).await;
    }
}

impl<W, Wr, F> FailureWriter<W> for Repeat<W, Wr, F>
where
    Wr: FailureWriter<W>,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        self.writer.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.writer.parsing_errors()
    }
}

impl<W, Wr, F> Repeat<W, Wr, F> {
    /// Creates [`Writer`] for re-outputting events at the end in case `filter`
    /// returns `true`.
    ///
    /// [`Skipped`]: event::Step::Skipped
    pub fn new(writer: Wr, filter: F) -> Self {
        Self {
            writer,
            filter,
            events: Vec::new(),
        }
    }
}

impl<W, Wr> Repeat<W, Wr> {
    /// Creates [`Writer`] for re-outputting [`Skipped`] events at the end.
    ///
    /// [`Skipped`]: event::Step::Skipped
    pub fn skipped(writer: Wr) -> Self {
        use event::{Cucumber, Feature, Rule, Scenario, Step};

        Self {
            writer,
            filter: |ev| {
                matches!(
                    ev,
                    Ok(Cucumber::Feature(
                        _,
                        Feature::Rule(
                            _,
                            Rule::Scenario(_, Scenario::Step(_, Step::Skipped))
                        ) | Feature::Scenario(
                            _,
                            Scenario::Step(_, Step::Skipped)
                        )
                    ))
                )
            },
            events: Vec::new(),
        }
    }

    /// Creates [`Writer`] for re-outputting [`Failed`] events and [`Parser`]
    /// errors at the end.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Parser`]: crate::Parser
    pub fn failed(writer: Wr) -> Self {
        use event::{Cucumber, Feature, Rule, Scenario, Step};

        Self {
            writer,
            filter: |ev| {
                matches!(
                    ev,
                    Ok(Cucumber::Feature(
                        _,
                        Feature::Rule(
                            _,
                            Rule::Scenario(
                                _,
                                Scenario::Step(_, Step::Failed(..))
                            )
                        ) | Feature::Scenario(
                            _,
                            Scenario::Step(_, Step::Failed(..))
                        )
                    )) | Err(_)
                )
            },
            events: Vec::new(),
        }
    }
}
