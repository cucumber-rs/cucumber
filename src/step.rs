// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Definitions for a [`Collection`] which is used to store [`Step`] [`Fn`]s and
//! corresponding [`Regex`] patterns.
//!
//! [`Step`]: gherkin::Step

use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    iter,
    ops::Deref,
};

use futures::future::LocalBoxFuture;
use gherkin::StepType;
use regex::{CaptureLocations, Regex};

/// Alias for a [`gherkin::Step`] function that returns a [`LocalBoxFuture`].
pub type Step<World> =
    for<'a> fn(&'a mut World, Context) -> LocalBoxFuture<'a, ()>;

/// Collection of [`Step`]s.
///
/// Every [`Step`] should be matched by exactly 1 [`Regex`]. Otherwise there are
/// no guarantees that [`Step`]s will be matched deterministically from run to
/// run.
pub struct Collection<World> {
    given: HashMap<HashableRegex, Step<World>>,
    when: HashMap<HashableRegex, Step<World>>,
    then: HashMap<HashableRegex, Step<World>>,
}

impl<World> fmt::Debug for Collection<World> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Collection")
            .field(
                "given",
                &self
                    .given
                    .iter()
                    .map(|(re, step)| (re, format!("{:p}", step)))
                    .collect::<HashMap<_, _>>(),
            )
            .field(
                "when",
                &self
                    .when
                    .iter()
                    .map(|(re, step)| (re, format!("{:p}", step)))
                    .collect::<HashMap<_, _>>(),
            )
            .field(
                "then",
                &self
                    .then
                    .iter()
                    .map(|(re, step)| (re, format!("{:p}", step)))
                    .collect::<HashMap<_, _>>(),
            )
            .finish()
    }
}

impl<World> Default for Collection<World> {
    fn default() -> Self {
        Self {
            given: HashMap::new(),
            when: HashMap::new(),
            then: HashMap::new(),
        }
    }
}

impl<World> Collection<World> {
    /// Creates a new empty [`Collection`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a [Given] [`Step`] matching the given `regex`.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    #[must_use]
    pub fn given(mut self, regex: Regex, step: Step<World>) -> Self {
        let _ = self.given.insert(regex.into(), step);
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#when
    #[must_use]
    pub fn when(mut self, regex: Regex, step: Step<World>) -> Self {
        let _ = self.when.insert(regex.into(), step);
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    #[must_use]
    pub fn then(mut self, regex: Regex, step: Step<World>) -> Self {
        let _ = self.then.insert(regex.into(), step);
        self
    }

    /// Returns a [`Step`] function matching the given [`gherkin::Step`],
    /// if any.
    #[must_use]
    pub fn find(
        &self,
        step: &gherkin::Step,
    ) -> Option<(&Step<World>, CaptureLocations, Context)> {
        let collection = match step.ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then,
        };

        let (whole_match, locations, step_fn) =
            collection.iter().find_map(|(re, step_fn)| {
                let mut locations = re.capture_locations();
                re.captures_read(&mut locations, &step.value)
                    .map(|m| (m, locations, step_fn))
            })?;

        let matches = iter::once(whole_match.as_str().to_owned())
            .chain((1..locations.len()).map(|group_id| {
                locations
                    .get(group_id)
                    .map_or("", |(s, e)| &step.value[s..e])
                    .to_owned()
            }))
            .collect();

        Some((
            step_fn,
            locations,
            Context {
                step: step.clone(),
                matches,
            },
        ))
    }
}

/// Context for a [`Step`] function execution.
#[derive(Debug)]
pub struct Context {
    /// [`Step`] matched to a [`Step`] function.
    ///
    /// [`Step`]: gherkin::Step
    pub step: gherkin::Step,

    /// [`Regex`] matches of a [`Step::value`].
    ///
    /// [`Step::value`]: gherkin::Step::value
    pub matches: Vec<String>,
}

/// [`Regex`] wrapper to store inside a [`LinkedHashMap`].
#[derive(Clone, Debug)]
struct HashableRegex(Regex);

impl From<Regex> for HashableRegex {
    fn from(re: Regex) -> Self {
        Self(re)
    }
}

impl Hash for HashableRegex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

impl PartialEq for HashableRegex {
    fn eq(&self, other: &HashableRegex) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for HashableRegex {}

impl Deref for HashableRegex {
    type Target = Regex;

    fn deref(&self) -> &Regex {
        &self.0
    }
}
