// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Writer`]-wrapper for transforming [`Skipped`] [`Step`]s into [`Failed`].
//!
//! [`Failed`]: event::Step::Failed
//! [`Skipped`]: event::Step::Skipped
//! [`Step`]: gherkin::Step

use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Deref;

use crate::{event, parser, writer, Event, World, Writer};

/// [`Writer`]-wrapper for transforming [`Skipped`] [`Step`]s into [`Failed`].
///
/// [`Failed`]: event::Step::Failed
/// [`Skipped`]: event::Step::Skipped
/// [`Step`]: gherkin::Step
#[derive(Debug, Deref)]
pub struct FailOnSkipped<W, F = SkipFn> {
    /// Original [`Writer`] to pass transformed event into.
    #[deref]
    pub writer: W,

    /// [`Fn`] to determine whether [`Skipped`] test should be considered as
    /// [`Failed`] or not.
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Skipped`]: event::Step::Skipped
    should_fail: F,
}

/// Alias for a [`fn`] used to determine whether [`Skipped`] test should be
/// considered as [`Failed`] or not.
///
/// [`Failed`]: event::Step::Failed
/// [`Skipped`]: event::Step::Skipped
pub type SkipFn =
    fn(&gherkin::Feature, Option<&gherkin::Rule>, &gherkin::Scenario) -> bool;

#[async_trait(?Send)]
impl<W, Wr, F> Writer<W> for FailOnSkipped<Wr, F>
where
    W: World,
    F: Fn(
        &gherkin::Feature,
        Option<&gherkin::Rule>,
        &gherkin::Scenario,
    ) -> bool,
    Wr: for<'val> writer::Arbitrary<'val, W, String>,
{
    type Cli = Wr::Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        use event::{
            Cucumber, Feature, Rule, Scenario, Step, StepError::Panic,
        };

        let map_failed = |f: Arc<_>, r: Option<Arc<_>>, sc: Arc<_>, st: _| {
            let ev = if (self.should_fail)(&f, r.as_deref(), &sc) {
                Step::Failed(None, None, Panic(Arc::new("not allowed to skip")))
            } else {
                Step::Skipped
            };

            Cucumber::scenario(f, r, sc, Scenario::Step(st, ev))
        };

        let event = event.map(|outer| {
            outer.map(|ev| match ev {
                Cucumber::Feature(
                    f,
                    Feature::Rule(
                        r,
                        Rule::Scenario(sc, Scenario::Step(st, Step::Skipped)),
                    ),
                ) => map_failed(f, Some(r), sc, st),
                Cucumber::Feature(
                    f,
                    Feature::Scenario(sc, Scenario::Step(st, Step::Skipped)),
                ) => map_failed(f, None, sc, st),
                Cucumber::Started
                | Cucumber::Feature(..)
                | Cucumber::Finished => ev,
            })
        });

        self.writer.handle_event(event, cli).await;
    }
}

#[async_trait(?Send)]
impl<'val, W, Wr, Val, F> writer::Arbitrary<'val, W, Val>
    for FailOnSkipped<Wr, F>
where
    W: World,
    Self: Writer<W>,
    Wr: writer::Arbitrary<'val, W, Val>,
    Val: 'val,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.writer.write(val).await;
    }
}

impl<W, Wr, F> writer::Failure<W> for FailOnSkipped<Wr, F>
where
    Wr: writer::Failure<W>,
    Self: Writer<W>,
{
    fn failed_steps(&self) -> usize {
        self.writer.failed_steps()
    }

    fn parsing_errors(&self) -> usize {
        self.writer.parsing_errors()
    }

    fn hook_errors(&self) -> usize {
        self.writer.hook_errors()
    }
}

impl<Wr: writer::Normalized, F> writer::Normalized for FailOnSkipped<Wr, F> {}

impl<Writer> From<Writer> for FailOnSkipped<Writer> {
    fn from(writer: Writer) -> Self {
        Self {
            writer,
            should_fail: |_, _, sc| {
                !sc.tags.iter().any(|tag| tag == "allow_skipped")
            },
        }
    }
}

impl<Writer> FailOnSkipped<Writer> {
    /// Wraps the given [`Writer`] in a new [`FailOnSkipped`] one.
    #[must_use]
    pub fn new(writer: Writer) -> Self {
        Self::from(writer)
    }

    /// Wraps the given [`Writer`] in a new [`FailOnSkipped`] one with the given
    /// `predicate` indicating when a [`Skipped`] [`Step`] is considered
    /// [`Failed`].
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Skipped`]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    #[must_use]
    pub fn with<P>(writer: Writer, predicate: P) -> FailOnSkipped<Writer, P>
    where
        P: Fn(
            &gherkin::Feature,
            Option<&gherkin::Rule>,
            &gherkin::Scenario,
        ) -> bool,
    {
        FailOnSkipped {
            writer,
            should_fail: predicate,
        }
    }
}
