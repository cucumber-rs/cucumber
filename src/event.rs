// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Key occurrences in a lifecycle of a [Cucumber] execution.
//!
//! The top-level enum here is [`Cucumber`].
//!
//! Each event enum contains variants indicating what stage of execution
//! [`Runner`] is at, and variants with detailed content about the precise
//! sub-event.
//!
//! [`Runner`]: crate::Runner
//!
//! [Cucumber]: https://cucumber.io

use std::{any::Any, sync::Arc};

/// Alias for a [`catch_unwind()`] error.
///
/// [`catch_unwind()`]: std::panic::catch_unwind()
pub type Info = Box<dyn Any + Send + 'static>;

/// Top-level [Cucumber] run event.
///
/// [Cucumber]: https://cucumber.io
#[derive(Debug)]
pub enum Cucumber<World> {
    /// Event for a [`Cucumber`] execution started.
    Started,

    /// [`Feature`] event.
    Feature(Arc<gherkin::Feature>, Feature<World>),

    /// Failed to parse a [`Feature`] file.
    ParsingError(gherkin::ParseFileError),

    /// Event for a [`Cucumber`] execution finished.
    Finished,
}

impl<World> Cucumber<World> {
    /// Constructs an event of a [`Feature`] being started.
    ///
    /// [`Feature`]: gherkin::Feature
    #[must_use]
    pub fn feature_started(feat: Arc<gherkin::Feature>) -> Self {
        Self::Feature(feat, Feature::Started)
    }

    /// Constructs an event of a [`Rule`] being started.
    ///
    /// [`Rule`]: gherkin::Rule
    #[must_use]
    pub fn rule_started(
        feat: Arc<gherkin::Feature>,
        rule: Arc<gherkin::Rule>,
    ) -> Self {
        Self::Feature(feat, Feature::Rule(rule, Rule::Started))
    }

    /// Constructs an event of a [`Feature`] being finished.
    ///
    /// [`Feature`]: gherkin::Feature
    #[must_use]
    pub fn feature_finished(feat: Arc<gherkin::Feature>) -> Self {
        Self::Feature(feat, Feature::Finished)
    }

    /// Constructs an event of a [`Rule`] being finished.
    ///
    /// [`Rule`]: gherkin::Rule
    #[must_use]
    pub fn rule_finished(
        feat: Arc<gherkin::Feature>,
        rule: Arc<gherkin::Rule>,
    ) -> Self {
        Self::Feature(feat, Feature::Rule(rule, Rule::Finished))
    }

    /// Constructs a [`Cucumber`] event from the given [`Scenario`] event.
    #[must_use]
    pub fn scenario(
        feat: Arc<gherkin::Feature>,
        rule: Option<Arc<gherkin::Rule>>,
        scenario: Arc<gherkin::Scenario>,
        event: Scenario<World>,
    ) -> Self {
        #[allow(clippy::option_if_let_else)] // use of moved value: `event`
        if let Some(r) = rule {
            Self::Feature(
                feat,
                Feature::Rule(r, Rule::Scenario(scenario, event)),
            )
        } else {
            Self::Feature(feat, Feature::Scenario(scenario, event))
        }
    }
}

/// Event specific to a particular [Feature]
///
/// [Feature]: https://cucumber.io/docs/gherkin/reference/#feature
#[derive(Debug)]
pub enum Feature<World> {
    /// [`Feature`] execution being started.
    ///
    /// [`Feature`]: gherkin::Feature
    Started,

    /// [`Rule`] event.
    Rule(Arc<gherkin::Rule>, Rule<World>),

    /// [`Scenario`] event.
    Scenario(Arc<gherkin::Scenario>, Scenario<World>),

    /// [`Feature`] execution being finished.
    ///
    /// [`Feature`]: gherkin::Feature
    Finished,
}

/// Event specific to a particular [Rule].
///
/// [Rule]: https://cucumber.io/docs/gherkin/reference/#rule
#[derive(Debug)]
pub enum Rule<World> {
    /// [`Rule`] execution being started.
    ///
    /// [`Rule`]: gherkin::Rule
    Started,

    /// [`Scenario`] event.
    Scenario(Arc<gherkin::Scenario>, Scenario<World>),

    /// [`Rule`] execution being finished.
    ///
    /// [`Rule`]: gherkin::Rule
    Finished,
}

/// Event specific to a particular [Scenario].
///
/// [Scenario]: https://cucumber.io/docs/gherkin/reference/#example
#[derive(Debug)]
pub enum Scenario<World> {
    /// [`Scenario`] execution being started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    Started,

    /// [`Background`] [`Step`] event.
    ///
    /// [`Background`]: gherkin::Background
    Background(Arc<gherkin::Step>, Step<World>),

    /// [`Step`] event.
    Step(Arc<gherkin::Step>, Step<World>),

    /// [`Scenario`] execution being finished.
    ///
    /// [`Scenario`]: gherkin::Scenario
    Finished,
}

impl<World> Scenario<World> {
    /// Event of a [`Step`] being started.
    ///
    /// [`Step`]: gherkin::Step
    pub fn step_started(step: Arc<gherkin::Step>) -> Self {
        Self::Step(step, Step::Started)
    }

    /// Event of a [`Background`] [`Step`] being started.
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub fn background_step_started(step: Arc<gherkin::Step>) -> Self {
        Self::Background(step, Step::Started)
    }

    /// Event of a passed [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    pub fn step_passed(step: Arc<gherkin::Step>) -> Self {
        Self::Step(step, Step::Passed)
    }

    /// Event of a passed [`Background`] [`Step`].
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub fn background_step_passed(step: Arc<gherkin::Step>) -> Self {
        Self::Background(step, Step::Passed)
    }

    /// Event of a skipped [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    pub fn step_skipped(step: Arc<gherkin::Step>) -> Self {
        Self::Step(step, Step::Skipped)
    }
    /// Event of a skipped [`Background`] [`Step`].
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub fn background_step_skipped(step: Arc<gherkin::Step>) -> Self {
        Self::Background(step, Step::Skipped)
    }

    /// Event of a failed [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    pub fn step_failed(
        step: Arc<gherkin::Step>,
        world: World,
        info: Info,
    ) -> Self {
        Self::Step(step, Step::Failed(world, info))
    }

    /// Event of a failed [`Background`] [`Step`].
    ///
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub fn background_step_failed(
        step: Arc<gherkin::Step>,
        world: World,
        info: Info,
    ) -> Self {
        Self::Background(step, Step::Failed(world, info))
    }
}

/// Event specific to a particular [Step].
///
/// [Step]: https://cucumber.io/docs/gherkin/reference/#step
#[derive(Debug)]
pub enum Step<World> {
    /// Event of a [`Step`] execution being started.
    ///
    /// [`Step`]: gherkin::Step
    Started,

    /// Event of a [`Step`] being being skipped.
    ///
    /// That means there is no [`Regex`] matching [`Step`] in a
    /// [`step::Collection`].
    ///
    /// [`Regex`]: regex::Regex
    /// [`Step`]: gherkin::Step
    /// [`step::Collection`]: crate::step::Collection
    Skipped,

    /// Event of a passed [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    Passed,

    /// Event of a failed [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    Failed(World, Info),
}
