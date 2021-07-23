//! Definitions for [`Collection`] which is used to store [`Step`] [`Fn`]s and
//! corresponding [`Regex`] patterns.
//!
//! [`Step`]: gherkin::Step

use std::{
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
};

use futures::future::LocalBoxFuture;
use gherkin::StepType;
use regex::Regex;

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

impl<World> Debug for Collection<World> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
    /// Creates empty [`Collection`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds [`Step`] that matched with [Given] steps which [`Step::value`]
    /// matches `regex`.
    ///
    /// [`Step::value`]: gherkin::Step::value
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference/#given
    pub fn given(mut self, regex: Regex, step: Step<World>) -> Self {
        let _ = self.given.insert(regex.into(), step);
        self
    }

    /// Adds [`Step`] that matched with [When] steps which [`Step::value`]
    /// matches `regex`.
    ///
    /// [`Step::value`]: gherkin::Step::value
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference/#when
    pub fn when(mut self, regex: Regex, step: Step<World>) -> Self {
        let _ = self.when.insert(regex.into(), step);
        self
    }

    /// Adds [`Step`] that matched with [Then] steps which [`Step::value`]
    /// matches `regex`.
    ///
    /// [`Step::value`]: gherkin::Step::value
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference/#then
    pub fn then(mut self, regex: Regex, step: Step<World>) -> Self {
        let _ = self.then.insert(regex.into(), step);
        self
    }

    /// Returns [`Step`] matching the [`Step::value`], if present.
    ///
    /// [`Step::value`]: gherkin::Step::value
    #[must_use]
    pub fn find(&self, step: gherkin::Step) -> Option<(&Step<World>, Context)> {
        let collection = match step.ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then,
        };

        let (captures, step_fn) =
            collection.iter().find_map(|(re, step_fn)| {
                re.captures(&step.value).map(|c| (c, step_fn))
            })?;

        let matches = captures
            .iter()
            .map(|c| c.map(|c| c.as_str().to_owned()).unwrap_or_default())
            .collect();

        Some((step_fn, Context { step, matches }))
    }
}

/// Alias for a [`fn`] that returns [`LocalBoxFuture`].
pub type Step<World> =
    for<'a> fn(&'a mut World, Context) -> LocalBoxFuture<'a, ()>;

/// Context for a [`Fn`] execution.
#[derive(Debug)]
pub struct Context {
    /// [`Step`] matched to a [`Fn`].
    ///
    /// [`Step`]: gherkin::Step
    pub step: gherkin::Step,

    /// [`Regex`] matches of a [`Step::value`].
    ///
    /// [`Step::value`]: gherkin::Step::value
    pub matches: Vec<String>,
}

/// [`Regex`] wrapper to store inside [`LinkedHashMap`].
#[derive(Clone, Debug)]
struct HashableRegex(Regex);

impl From<Regex> for HashableRegex {
    fn from(re: Regex) -> Self {
        HashableRegex(re)
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
