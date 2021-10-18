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
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    fmt,
    hash::{Hash, Hasher},
    iter,
    path::PathBuf,
};

use derive_more::{Deref, DerefMut};
use futures::future::LocalBoxFuture;
use gherkin::StepType;
use regex::Regex;

/// Alias for a [`gherkin::Step`] function that returns a [`LocalBoxFuture`].
pub type Step<World> =
    for<'a> fn(&'a mut World, Context) -> LocalBoxFuture<'a, ()>;

/// Alias for a [`Step`] with [`regex::CaptureLocations`] and [`Context`]
/// returned by [`Collection::find()`].
pub type WithContext<'me, World> =
    (&'me Step<World>, regex::CaptureLocations, Context);

/// Collection of [`Step`]s.
///
/// Every [`Step`] has to be matched by exactly 1 [`Regex`].
pub struct Collection<World> {
    given: BTreeMap<(HashableRegex, Option<Location>), Step<World>>,
    when: BTreeMap<(HashableRegex, Option<Location>), Step<World>>,
    then: BTreeMap<(HashableRegex, Option<Location>), Step<World>>,
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
            given: BTreeMap::new(),
            when: BTreeMap::new(),
            then: BTreeMap::new(),
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
    pub fn given(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        let _ = self.given.insert((regex.into(), loc), step);
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#when
    #[must_use]
    pub fn when(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        let _ = self.when.insert((regex.into(), loc), step);
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    #[must_use]
    pub fn then(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        let _ = self.then.insert((regex.into(), loc), step);
        self
    }

    /// Returns a [`Step`] function matching the given [`gherkin::Step`], if
    /// any.
    ///
    /// # Errors
    ///
    /// If the given [`gherkin::Step`] matches multiple [`Regex`]es.
    pub fn find(
        &self,
        step: &gherkin::Step,
    ) -> Result<Option<WithContext<'_, World>>, AmbiguousMatchError> {
        let collection = match step.ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then,
        };

        let mut matches = collection
            .iter()
            .filter_map(|((re, loc), step_fn)| {
                let mut captures = re.capture_locations();
                re.captures_read(&mut captures, &step.value)
                    .map(|m| (re, loc, m, captures, step_fn))
            })
            .collect::<Vec<_>>();

        let (_, _, whole_match, captures, step_fn) = match matches.len() {
            0 => return Ok(None),
            1 => matches.pop().unwrap(),
            _ => {
                return Err(AmbiguousMatchError {
                    possible_matches: matches
                        .into_iter()
                        .map(|(re, loc, ..)| (re.clone(), loc.clone()))
                        .collect(),
                })
            }
        };

        let matches = iter::once(whole_match.as_str().to_owned())
            .chain((1..captures.len()).map(|group_id| {
                captures
                    .get(group_id)
                    .map_or("", |(s, e)| &step.value[s..e])
                    .to_owned()
            }))
            .collect();

        Ok(Some((
            step_fn,
            captures,
            Context {
                step: step.clone(),
                matches,
            },
        )))
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

/// Error of a [`gherkin::Step`] matching multiple [`Step`] [`Regex`]es inside a
/// [`Collection`].
#[derive(Clone, Debug)]
pub struct AmbiguousMatchError {
    /// Possible [`Regex`]es the [`gherkin::Step`] matches.
    pub possible_matches: Vec<(HashableRegex, Option<Location>)>,
}

/// Location of a [`Step`] [`fn`] automatically filled by a proc macro.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    /// Path to the file where [`Step`] [`fn`] is located.
    pub path: PathBuf,

    /// Line of the file where [`Step`] [`fn`] is located.
    pub line: u32,

    /// Column of the file where [`Step`] [`fn`] is located.
    pub column: u32,
}

/// [`Regex`] wrapper implementing [`Eq`], [`Ord`] and [`Hash`].
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct HashableRegex(Regex);

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

impl PartialOrd for HashableRegex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.as_str().partial_cmp(other.0.as_str())
    }
}

impl Ord for HashableRegex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_str().cmp(other.0.as_str())
    }
}
