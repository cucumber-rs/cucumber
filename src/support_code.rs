//! Run-frozen descriptions of the definitions a runner actually resolves.
//!
//! Records contain actual compiled regular expressions and, when retained by
//! a registration, original expressions and validated parameter layouts.
//! Original expressions are never guessed from regexes. Use a fresh [`Store`]
//! for each run.

mod expression;
pub use expression::{ArgumentGroup, ExpressionMetadata, MatchArgument, MetadataError, ParameterDescriptor, RegexMetadata};

use std::sync::{Arc, RwLock};

use regex::Regex;

use crate::step::{Location, Matching};

/// An actual collection entry, without its executable function pointer.
#[derive(Clone, Debug)]
pub struct Definition {
    id: String,
    keyword: gherkin::StepType,
    regex: Regex,
    location: Option<Location>,
    expression: Option<ExpressionMetadata>,
    regular_expression: Option<RegexMetadata>,
}

impl Definition {
    /// Identity stable for the lifetime of this run's snapshot.
    #[must_use]
    pub fn id(&self) -> &str { &self.id }

    /// Keyword under which this definition was registered.
    #[must_use]
    pub const fn keyword(&self) -> gherkin::StepType { self.keyword }

    /// The actual compiled regular expression used by the runner.
    #[must_use]
    pub fn regex(&self) -> &Regex { &self.regex }

    /// Actual original expression and complete capture metadata, when retained.
    #[must_use]
    pub fn expression_metadata(&self) -> Option<&ExpressionMetadata> { self.expression.as_ref() }

    /// Explicit raw-regex argument mapping, when supplied by the registration.
    #[must_use]
    pub fn regular_expression_metadata(&self) -> Option<&RegexMetadata> { self.regular_expression.as_ref() }

    /// The registration's source location, if the caller supplied one.
    #[must_use]
    pub const fn location(&self) -> Option<Location> { self.location }
}

/// Immutable registry and matching policy captured immediately before execution.
#[derive(Clone, Debug)]
pub struct Snapshot {
    matching: Matching,
    definitions: Vec<Definition>,
}

impl Snapshot {
    pub(crate) fn new(matching: Matching,
        registrations: impl IntoIterator<Item = (gherkin::StepType, Regex, Option<Location>, Option<ExpressionMetadata>, Option<RegexMetadata>)>,
    ) -> Self {
        let mut registrations = registrations.into_iter().collect::<Vec<_>>();
        let rank = |keyword| match keyword {
            gherkin::StepType::Given => 0,
            gherkin::StepType::When => 1,
            gherkin::StepType::Then => 2,
        };
        registrations.sort_by(|left, right| {
            (rank(left.0), left.1.as_str(), left.2)
                .cmp(&(rank(right.0), right.1.as_str(), right.2))
        });
        let definitions = registrations.into_iter().enumerate()
            .map(|(index, (keyword, regex, location, expression, regular_expression))| Definition {
                id: format!("step-definition-{index}"), keyword, regex, location, expression, regular_expression,
            }).collect();
        Self { matching, definitions }
    }

    /// The policy used for actual execution of this collection.
    #[must_use]
    pub const fn matching(&self) -> Matching { self.matching }

    /// All final entries, including definitions not matched by any executed step.
    #[must_use]
    pub fn definitions(&self) -> &[Definition] { &self.definitions }

    /// All matching registration identities, including ambiguous matches.
    ///
    /// An empty result means no definition matched. Keyword-independent mode
    /// preserves separate entries under different keywords, just as execution does.
    #[must_use]
    pub fn matching_ids(&self, step: &gherkin::Step) -> Vec<&str> {
        self.matching_definitions(step).into_iter().map(Definition::id).collect()
    }

    /// Actual matching entries in the same stable order as [`Self::matching_ids`].
    #[must_use]
    pub fn matching_definitions(&self, step: &gherkin::Step) -> Vec<&Definition> {
        self.definitions.iter().filter(|definition| {
            (self.matching == Matching::KeywordIndependent || definition.keyword == step.ty)
                && definition.regex.is_match(&step.value)
        }).collect()
    }
}

/// Shared handoff from a runner to its reporting consumers.
///
/// An absent snapshot means the runner did not supply metadata, which differs
/// from a supported runner publishing an empty collection. Clones share the
/// current entry; retained snapshots remain immutable. Independent or concurrent
/// runs must use fresh stores, as [`crate::Cucumber::with_messages()`] does.
#[derive(Clone, Debug, Default)]
pub struct Store(Arc<RwLock<Option<Arc<Snapshot>>>>);

impl Store {
    /// Creates an empty handoff for one run.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Returns the immutable registry published by the runner, if supported.
    #[must_use]
    pub fn get(&self) -> Option<Arc<Snapshot>> {
        self.0.read().expect("support-code store lock poisoned").clone()
    }

    /// Publishes the final collection before any execution events are produced.
    ///
    /// Custom runners may use this only when the snapshot reflects their actual
    /// registrations and matching policy. It is never inferred from outcomes.
    pub fn publish(&self, snapshot: Snapshot) {
        *self.0.write().expect("support-code store lock poisoned") = Some(Arc::new(snapshot));
    }
}
