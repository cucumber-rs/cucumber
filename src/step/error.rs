//! Error types for step matching and execution.
//!
//! This module provides error types that can occur during step matching,
//! such as when a step matches multiple regex patterns.

use std::fmt;

use derive_more::with_trait::Error;
use itertools::Itertools as _;

use super::{location::Location, regex::HashableRegex};

/// Error of a [`gherkin::Step`] matching multiple [`Step`] [`Regex`]es inside a
/// [`Collection`].
///
/// [`Collection`]: super::Collection
#[derive(Clone, Debug, Error)]
pub struct AmbiguousMatchError {
    /// Possible [`Regex`]es the [`gherkin::Step`] matches.
    pub possible_matches: Vec<(HashableRegex, Option<Location>)>,
}

impl AmbiguousMatchError {
    /// Creates a new [`AmbiguousMatchError`] with the given possible matches.
    #[must_use]
    pub fn new(possible_matches: Vec<(HashableRegex, Option<Location>)>) -> Self {
        Self { possible_matches }
    }

    /// Returns a reference to the possible matches.
    #[must_use]
    pub fn possible_matches(&self) -> &[(HashableRegex, Option<Location>)] {
        &self.possible_matches
    }

    /// Returns the number of possible matches.
    #[must_use]
    pub fn match_count(&self) -> usize {
        self.possible_matches.len()
    }

    /// Returns an iterator over the regex patterns that matched.
    pub fn patterns(&self) -> impl Iterator<Item = &str> + '_ {
        self.possible_matches.iter().map(|(regex, _)| regex.as_str())
    }

    /// Returns an iterator over the locations of the matching steps.
    pub fn locations(&self) -> impl Iterator<Item = Option<&Location>> + '_ {
        self.possible_matches.iter().map(|(_, loc)| loc.as_ref())
    }

    /// Returns a sorted copy of the possible matches.
    #[must_use]
    pub fn sorted_matches(&self) -> Vec<(HashableRegex, Option<Location>)> {
        self.possible_matches.iter().cloned().sorted().collect()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn create_test_matches() -> Vec<(HashableRegex, Option<Location>)> {
        vec![
            (
                HashableRegex::from(Regex::new(r"I have (\d+) cucumbers").unwrap()),
                Some(Location::new("src/steps.rs", 10, 5)),
            ),
            (
                HashableRegex::from(Regex::new(r"I have \d+ cucumbers").unwrap()),
                Some(Location::new("src/more_steps.rs", 20, 10)),
            ),
            (
                HashableRegex::from(Regex::new(r"I have .+ cucumbers").unwrap()),
                None,
            ),
        ]
    }

    #[test]
    fn ambiguous_match_error_new_creates_error() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches.clone());
        assert_eq!(error.possible_matches.len(), matches.len());
        assert_eq!(error.possible_matches, matches);
    }

    #[test]
    fn ambiguous_match_error_possible_matches_returns_reference() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches.clone());
        assert_eq!(error.possible_matches(), &matches);
    }

    #[test]
    fn ambiguous_match_error_match_count_returns_correct_count() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches);
        assert_eq!(error.match_count(), 3);
    }

    #[test]
    fn ambiguous_match_error_patterns_returns_regex_patterns() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches);
        
        let patterns: Vec<&str> = error.patterns().collect();
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&r"I have (\d+) cucumbers"));
        assert!(patterns.contains(&r"I have \d+ cucumbers"));
        assert!(patterns.contains(&r"I have .+ cucumbers"));
    }

    #[test]
    fn ambiguous_match_error_locations_returns_locations() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches);
        
        let locations: Vec<Option<&Location>> = error.locations().collect();
        assert_eq!(locations.len(), 3);
        
        // First two should have locations, third should be None
        assert!(locations[0].is_some());
        assert!(locations[1].is_some());
        assert!(locations[2].is_none());
        
        if let Some(loc) = locations[0] {
            assert_eq!(loc.path(), "src/steps.rs");
            assert_eq!(loc.line(), 10);
        }
        
        if let Some(loc) = locations[1] {
            assert_eq!(loc.path(), "src/more_steps.rs");
            assert_eq!(loc.line(), 20);
        }
    }

    #[test]
    fn ambiguous_match_error_sorted_matches_returns_sorted_copy() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches);
        
        let sorted = error.sorted_matches();
        assert_eq!(sorted.len(), 3);
        
        // Should be sorted by regex pattern (alphabetically)
        let patterns: Vec<&str> = sorted.iter().map(|(r, _)| r.as_str()).collect();
        assert!(patterns[0] < patterns[1]);
        assert!(patterns[1] < patterns[2]);
    }

    #[test]
    fn ambiguous_match_error_display_works() {
        let matches = vec![
            (
                HashableRegex::from(Regex::new(r"pattern1").unwrap()),
                Some(Location::new("src/test.rs", 10, 5)),
            ),
            (
                HashableRegex::from(Regex::new(r"pattern2").unwrap()),
                None,
            ),
        ];
        
        let error = AmbiguousMatchError::new(matches);
        let display_output = format!("{}", error);
        
        assert!(display_output.contains("Possible matches:"));
        assert!(display_output.contains("pattern1"));
        assert!(display_output.contains("pattern2"));
        assert!(display_output.contains("src/test.rs:10:5"));
    }

    #[test]
    fn ambiguous_match_error_display_without_location() {
        let matches = vec![
            (
                HashableRegex::from(Regex::new(r"pattern").unwrap()),
                None,
            ),
        ];
        
        let error = AmbiguousMatchError::new(matches);
        let display_output = format!("{}", error);
        
        assert!(display_output.contains("Possible matches:"));
        assert!(display_output.contains("pattern"));
        // Should not contain " --> " when no location
        assert!(!display_output.contains(" --> "));
    }

    #[test]
    fn ambiguous_match_error_clone_works() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches.clone());
        let cloned = error.clone();
        
        assert_eq!(cloned.possible_matches, matches);
        assert_eq!(cloned.match_count(), error.match_count());
    }

    #[test]
    fn ambiguous_match_error_debug_works() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches);
        let debug_output = format!("{:?}", error);
        
        assert!(debug_output.contains("AmbiguousMatchError"));
        assert!(debug_output.contains("possible_matches"));
    }

    #[test]
    fn ambiguous_match_error_empty_matches() {
        let error = AmbiguousMatchError::new(vec![]);
        assert_eq!(error.match_count(), 0);
        assert!(error.patterns().collect::<Vec<_>>().is_empty());
        assert!(error.locations().collect::<Vec<_>>().is_empty());
        assert!(error.sorted_matches().is_empty());
    }

    #[test]
    fn ambiguous_match_error_is_error_trait() {
        let matches = create_test_matches();
        let error = AmbiguousMatchError::new(matches);
        
        // Should implement std::error::Error
        let _: &dyn std::error::Error = &error;
    }
}