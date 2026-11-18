// Copyright (c) 2018-2026  Brendan Molloy <brendan@bbqsrc.net>,
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

mod options;

pub use options::DefinitionOptions;

use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    iter,
};

use derive_more::with_trait::{Debug, Deref, DerefMut, Display, Error};
use futures::future::LocalBoxFuture;
use gherkin::StepType;
use itertools::Itertools as _;
use regex::Regex;

/// Alias for a [`gherkin::Step`] function that returns a [`LocalBoxFuture`].
pub type Step<World> =
    for<'a> fn(&'a mut World, Context) -> LocalBoxFuture<'a, ()>;

/// Alias for a [`Step`] with [`regex::CaptureLocations`], [`Location`] and
/// [`Context`] returned by [`Collection::find()`].
pub type WithContext<'me, World> =
    (&'me Step<World>, regex::CaptureLocations, Option<Location>, Context);

/// Policy for resolving a step definition from a Gherkin step.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Matching {
    /// Search only definitions registered under the step's keyword type.
    ///
    /// This is the backwards-compatible default.
    #[default]
    KeywordSpecific,

    /// Search all Given, When and Then definitions, regardless of keyword.
    ///
    /// Like Cucumber.js, multiple matching definitions are ambiguous even when
    /// registered under different keywords. This enables matching compatibility,
    /// not complete Cucumber.js runtime compatibility.
    KeywordIndependent,
}

struct Registration<W> {
    function: Step<W>,
    options: DefinitionOptions,
}
impl<W> Clone for Registration<W> {
    fn clone(&self) -> Self { Self { function: self.function, options: self.options.clone() } }
}

/// Collection of [`Step`]s.
///
/// Every [`Step`] has to match with exactly 1 [`Regex`].
#[derive(Debug)]
pub struct Collection<World> {
    /// Policy used by [`Self::find()`].
    matching: Matching,

    /// Collection of [Given] [`Step`]s.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[debug("{:?}",
        given.iter()
            .map(|(re, step)| (re, format!("{:p}", step.function)))
            .collect::<HashMap<_, _>>(),
    )]
    given: HashMap<(HashableRegex, Option<Location>), Registration<World>>,

    /// Collection of [When] [`Step`]s.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    #[debug("{:?}",
        when.iter()
            .map(|(re, step)| (re, format!("{:p}", step.function)))
            .collect::<HashMap<_, _>>(),
    )]
    when: HashMap<(HashableRegex, Option<Location>), Registration<World>>,

    /// Collection of [Then] [`Step`]s.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    #[debug("{:?}",
        then.iter()
            .map(|(re, step)| (re, format!("{:p}", step.function)))
            .collect::<HashMap<_, _>>(),
    )]
    then: HashMap<(HashableRegex, Option<Location>), Registration<World>>,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World> Clone for Collection<World> {
    fn clone(&self) -> Self {
        Self {
            matching: self.matching,
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
            matching: Matching::default(),
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

    /// Selects how [`Self::find()`] searches registered definitions.
    #[must_use]
    pub fn matching(mut self, matching: Matching) -> Self {
        self.matching = matching;
        self
    }

    /// Captures the actual final registrations and matching policy for reporting.
    ///
    /// Duplicate map keys have already been replaced by the collection's normal
    /// insertion semantics. No handler is executed and no source is inferred.
    #[must_use]
    pub fn support_code(&self) -> crate::support_code::Snapshot {
        crate::support_code::Snapshot::new(self.matching,
            [(StepType::Given, &self.given), (StepType::When, &self.when),
                (StepType::Then, &self.then)].into_iter().flat_map(|(keyword, collection)| {
                    collection.iter().map(move |((regex, location), registration)|
                        (keyword, regex.0.clone(), *location, registration.options.expression.clone(),
                            registration.options.regular_expression.clone()))
                }))
    }

    /// Adds a [Given] [`Step`] matching the given `regex`.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[must_use]
    pub fn given(
        self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        self.given_with_options(loc, regex, step, DefinitionOptions::default())
    }

    /// Inserts a Given registration with explicit options.
    ///
    /// Replacing the same regex/location key replaces both handler and options.
    #[must_use]
    pub fn given_with_options(mut self, loc: Option<Location>, regex: Regex,
        step: Step<World>, options: DefinitionOptions) -> Self
    {
        drop(self.given.insert((regex.into(), loc), Registration { function: step, options }));
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    #[must_use]
    pub fn when(
        self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        self.when_with_options(loc, regex, step, DefinitionOptions::default())
    }

    /// Inserts a When registration with explicit options.
    ///
    /// Replacing the same regex/location key replaces both handler and options.
    #[must_use]
    pub fn when_with_options(mut self, loc: Option<Location>, regex: Regex,
        step: Step<World>, options: DefinitionOptions) -> Self
    {
        drop(self.when.insert((regex.into(), loc), Registration { function: step, options }));
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    #[must_use]
    pub fn then(
        self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        self.then_with_options(loc, regex, step, DefinitionOptions::default())
    }

    /// Inserts a Then registration with explicit options.
    ///
    /// Replacing the same regex/location key replaces both handler and options.
    #[must_use]
    pub fn then_with_options(mut self, loc: Option<Location>, regex: Regex,
        step: Step<World>, options: DefinitionOptions) -> Self
    {
        drop(self.then.insert((regex.into(), loc), Registration { function: step, options }));
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
        self.find_with_options(step).map(|matched| matched.map(|(context, _)| context))
    }

    /// Resolves one registration and its options in the same matching pass.
    ///
    /// # Errors
    /// Returns the same ambiguity error as [`Self::find`].
    pub fn find_with_options(&self, step: &gherkin::Step)
        -> Result<Option<(WithContext<'_, World>, &DefinitionOptions)>, AmbiguousMatchError>
    {
        let collections = match self.matching {
            Matching::KeywordSpecific => [
                Some(match step.ty {
                    StepType::Given => &self.given,
                    StepType::When => &self.when,
                    StepType::Then => &self.then,
                }),
                None,
                None,
            ],
            Matching::KeywordIndependent => {
                [Some(&self.given), Some(&self.when), Some(&self.then)]
            }
        };

        let mut captures = collections
            .into_iter()
            .flatten()
            .flat_map(|collection| collection.iter())
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
                    });
                }
            };

        #[expect( // intentional
            clippy::string_slice,
            reason = "all indices are obtained from the source string"
        )]
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

        Ok(Some(((
            &step_fn.function,
            captures,
            *loc,
            Context { step: step.clone(), matches },
        ), &step_fn.options)))
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

impl Display for AmbiguousMatchError {
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
#[display("{path}:{line}:{column}")]
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
