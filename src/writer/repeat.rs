// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Writer`]-wrapper for re-outputting events at the end of an output.

use std::mem;

use async_trait::async_trait;
use derive_more::Deref;

use crate::{event, parser, writer, Event, World, Writer};

/// Alias for a [`fn`] predicate deciding whether an event should be
/// re-outputted or not.
pub type FilterEvent<W> =
    fn(&parser::Result<Event<event::Cucumber<W>>>) -> bool;

/// Wrapper for a [`Writer`] implementation for re-outputting events at the end
/// of an output, based on a filter predicated.
///
/// Useful for re-outputting [skipped] or [failed] [`Step`]s.
///
/// An underlying [`Writer`] has to be [`NonTransforming`].
///
/// [failed]: crate::WriterExt::repeat_failed
/// [skipped]: crate::WriterExt::repeat_skipped
/// [`NonTransforming`]: writer::NonTransforming
/// [`Step`]: gherkin::Step
#[derive(Debug, Deref)]
pub struct Repeat<W, Wr, F = FilterEvent<W>> {
    /// Original [`Writer`].
    #[deref]
    writer: Wr,

    /// Predicate to decide whether an event should be re-outputted or not.
    filter: F,

    /// Buffer of collected events for re-outputting.
    events: Vec<parser::Result<Event<event::Cucumber<W>>>>,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World, Wr: Clone, F: Clone> Clone for Repeat<World, Wr, F> {
    fn clone(&self) -> Self {
        Self {
            writer: self.writer.clone(),
            filter: self.filter.clone(),
            events: self.events.clone(),
        }
    }
}

#[async_trait(?Send)]
impl<W, Wr, F> Writer<W> for Repeat<W, Wr, F>
where
    W: World,
    Wr: Writer<W> + writer::NonTransforming,
    F: Fn(&parser::Result<Event<event::Cucumber<W>>>) -> bool,
{
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        if (self.filter)(&event) {
            self.events.push(event.clone());
        }

        let is_finished =
            matches!(event.as_deref(), Ok(event::Cucumber::Finished));

        self.writer.handle_event(event, cli).await;

        if is_finished {
            for ev in mem::take(&mut self.events) {
                self.writer.handle_event(ev, cli).await;
            }
        }
    }
}

#[warn(clippy::missing_trait_methods)]
#[async_trait(?Send)]
impl<'val, W, Wr, Val, F> writer::Arbitrary<'val, W, Val> for Repeat<W, Wr, F>
where
    W: World,
    Wr: writer::Arbitrary<'val, W, Val> + writer::NonTransforming,
    Val: 'val,
    F: Fn(&parser::Result<Event<event::Cucumber<W>>>) -> bool,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.writer.write(val).await;
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr, F> writer::Stats<W> for Repeat<W, Wr, F>
where
    Wr: writer::Stats<W> + writer::NonTransforming,
    Self: Writer<W>,
{
    fn passed_steps(&self) -> usize {
        self.writer.passed_steps()
    }

    fn skipped_steps(&self) -> usize {
        self.writer.skipped_steps()
    }

    fn failed_steps(&self) -> usize {
        self.writer.failed_steps()
    }

    fn retried_steps(&self) -> usize {
        self.writer.retried_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.writer.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.writer.hook_errors()
    }

    fn execution_has_failed(&self) -> bool {
        self.writer.execution_has_failed()
    }
}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr: writer::Normalized, F> writer::Normalized for Repeat<W, Wr, F> {}

#[warn(clippy::missing_trait_methods)]
impl<W, Wr, F> writer::Summarizable for Repeat<W, Wr, F> {}

impl<W, Wr, F> Repeat<W, Wr, F> {
    /// Creates a new [`Writer`] for re-outputting events at the end of an
    /// output in case the given `filter` predicated returns `true`.
    #[must_use]
    pub const fn new(writer: Wr, filter: F) -> Self {
        Self {
            writer,
            filter,
            events: Vec::new(),
        }
    }
}

impl<W, Wr> Repeat<W, Wr> {
    /// Creates [`Writer`] for re-outputting [`Skipped`] events at the end of
    /// an output.
    ///
    /// [`Skipped`]: event::Step::Skipped
    #[must_use]
    pub fn skipped(writer: Wr) -> Self {
        use event::{
            Cucumber, Feature, RetryableScenario, Rule, Scenario, Step,
        };

        Self {
            writer,
            filter: |ev| {
                matches!(
                    ev.as_deref(),
                    Ok(Cucumber::Feature(
                        _,
                        Feature::Rule(
                            _,
                            Rule::Scenario(
                                _,
                                RetryableScenario {
                                    event: Scenario::Step(_, Step::Skipped)
                                        | Scenario::Background(
                                            _,
                                            Step::Skipped
                                        ),
                                    ..
                                }
                            )
                        ) | Feature::Scenario(
                            _,
                            RetryableScenario {
                                event: Scenario::Step(_, Step::Skipped)
                                    | Scenario::Background(_, Step::Skipped),
                                ..
                            }
                        )
                    )),
                )
            },
            events: Vec::new(),
        }
    }

    /// Creates a [`Writer`] for re-outputting [`Failed`] events and [`Parser`]
    /// errors at the end of an output.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Parser`]: crate::Parser
    #[must_use]
    pub fn failed(writer: Wr) -> Self {
        use event::{
            Cucumber, Feature, Hook, RetryableScenario, Rule, Scenario, Step,
        };

        Self {
            writer,
            filter: |ev| {
                matches!(
                    ev.as_deref(),
                    Ok(Cucumber::Feature(
                        _,
                        Feature::Rule(
                            _,
                            Rule::Scenario(
                                _,
                                RetryableScenario {
                                    event: Scenario::Step(_, Step::Failed(..))
                                        | Scenario::Background(
                                            _,
                                            Step::Failed(..),
                                        )
                                        | Scenario::Hook(_, Hook::Failed(..)),
                                    ..
                                }
                            )
                        ) | Feature::Scenario(
                            _,
                            RetryableScenario {
                                event: Scenario::Step(_, Step::Failed(..))
                                    | Scenario::Background(_, Step::Failed(..))
                                    | Scenario::Hook(_, Hook::Failed(..)),
                                ..
                            },
                        )
                    )) | Err(_),
                )
            },
            events: Vec::new(),
        }
    }

    /// Returns the original [`Writer`], wrapped by this [`Repeat`] one.
    #[must_use]
    pub fn inner_writer(&self) -> &Wr {
        &self.writer
    }
}
