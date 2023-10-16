// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    iter,
};

use derive_more::{Deref, DerefMut, Display, Error};
use futures::future::LocalBoxFuture;
use gherkin::StepType;
use itertools::Itertools as _;
use regex::Regex;

/// Alias for a [`gherkin::Step`] function that returns a [`LocalBoxFuture`].
pub type Step<World> =
    for<'a> fn(&'a mut World, Context) -> LocalBoxFuture<'a, ()>;

/// Alias for a [`Step`] with [`regex::CaptureLocations`], [`Location`] and
/// [`Context`] returned by [`Collection::find()`].
pub type WithContext<'me, World> = (
    &'me Step<World>,
    regex::CaptureLocations,
    Option<Location>,
    Context,
);

/// Collection of [`Step`]s.
///
/// Every [`Step`] has to match with exactly 1 [`Regex`].
pub struct Collection<World> {
    /// Collection of [Given] [`Step`]s.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    given: HashMap<(HashableRegex, Option<Location>), Step<World>>,

    /// Collection of [When] [`Step`]s.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    when: HashMap<(HashableRegex, Option<Location>), Step<World>>,

    /// Collection of [Then] [`Step`]s.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    then: HashMap<(HashableRegex, Option<Location>), Step<World>>,
}

impl<World> fmt::Debug for Collection<World> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Collection")
            .field(
                "given",
                &self
                    .given
                    .iter()
                    .map(|(re, step)| (re, format!("{step:p}")))
                    .collect::<HashMap<_, _>>(),
            )
            .field(
                "when",
                &self
                    .when
                    .iter()
                    .map(|(re, step)| (re, format!("{step:p}")))
                    .collect::<HashMap<_, _>>(),
            )
            .field(
                "then",
                &self
                    .then
                    .iter()
                    .map(|(re, step)| (re, format!("{step:p}")))
                    .collect::<HashMap<_, _>>(),
            )
            .finish()
    }
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World> Clone for Collection<World> {
    fn clone(&self) -> Self {
        Self {
            given: self.given.clone(),
            when: self.when.clone(),
            then: self.then.clone(),
        }
    }
}

// Implemented manually to omit redundant `World: Default` trait bound, imposed
// by `#[derive(Default)]`.
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
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[must_use]
    pub fn given(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        _ = self.given.insert((regex.into(), loc), step);
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    #[must_use]
    pub fn when(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        _ = self.when.insert((regex.into(), loc), step);
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    #[must_use]
    pub fn then(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        _ = self.then.insert((regex.into(), loc), step);
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

        let mut captures = collection
            .iter()
            .filter_map(|((re, loc), step_fn)| {
                let mut captures = re.capture_locations();
                let names = re.capture_names();
                re.captures_read(&mut captures, &step.value)
                    .map(|m| (re, loc, m, captures, names, step_fn))
            })
            .collect::<Vec<_>>();

        let (_, loc, whole_match, captures, names, step_fn) =
            match captures.len() {
                0 => return Ok(None),
                // Instead of `.unwrap()` to avoid documenting `# Panics`.
                1 => captures.pop().unwrap_or_else(|| unreachable!()),
                _ => {
                    return Err(AmbiguousMatchError {
                        possible_matches: captures
                            .into_iter()
                            .map(|(re, loc, ..)| (re.clone(), *loc))
                            .sorted()
                            .collect(),
                    })
                }
            };

        // PANIC: Slicing is OK here, as all indices are obtained from the
        //        source string.
        #[allow(clippy::string_slice)]
        let matches = names
            .map(|opt| opt.map(str::to_owned))
            .zip(iter::once(whole_match.as_str().to_owned()).chain(
                (1..captures.len()).map(|group_id| {
                    captures
                        .get(group_id)
                        .map_or("", |(s, e)| &step.value[s..e])
                        .to_owned()
                }),
            ))
            .collect();

        Ok(Some((
            step_fn,
            captures,
            *loc,
            Context {
                step: step.clone(),
                matches,
            },
        )))
    }
}

/// Name of a capturing group inside a [`regex`].
pub type CaptureName = Option<String>;

/// Context for a [`Step`] function execution.
#[derive(Clone, Debug)]
pub struct Context {
    /// [`Step`] matched to a [`Step`] function.
    ///
    /// [`Step`]: gherkin::Step
    pub step: gherkin::Step,

    /// [`Regex`] matches of a [`Step::value`].
    ///
    /// [`Step::value`]: gherkin::Step::value
    pub matches: Vec<(CaptureName, String)>,
}

/// Error of a [`gherkin::Step`] matching multiple [`Step`] [`Regex`]es inside a
/// [`Collection`].
#[derive(Clone, Debug, Error)]
pub struct AmbiguousMatchError {
    /// Possible [`Regex`]es the [`gherkin::Step`] matches.
    pub possible_matches: Vec<(HashableRegex, Option<Location>)>,
}

impl fmt::Display for AmbiguousMatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Possible matches:")?;
        for (reg, loc_opt) in &self.possible_matches {
            write!(f, "\n{reg}")?;
            if let Some(loc) = loc_opt {
                write!(f, " --> {loc}")?;
            }
        }
        Ok(())
    }
}

/// Location of a [`Step`] [`fn`] automatically filled by a proc macro.
#[derive(Clone, Copy, Debug, Display, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[display(fmt = "{}:{}:{}", path, line, column)]
pub struct Location {
    /// Path to the file where [`Step`] [`fn`] is located.
    pub path: &'static str,

    /// Line of the file where [`Step`] [`fn`] is located.
    pub line: u32,

    /// Column of the file where [`Step`] [`fn`] is located.
    pub column: u32,
}

/// [`Regex`] wrapper implementing [`Eq`], [`Ord`] and [`Hash`].
#[derive(Clone, Debug, Deref, DerefMut, Display)]
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
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for HashableRegex {}

impl PartialOrd for HashableRegex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HashableRegex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.as_str().cmp(other.0.as_str())
    }
}
