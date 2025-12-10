//! Location tracking for step definitions.
//!
//! This module provides the [`Location`] struct for tracking the file
//! location where step functions are defined, which is automatically
//! filled by proc macros.

use derive_more::with_trait::{Debug, Display};
use std::hash::Hash;

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

impl Location {
    /// Creates a new [`Location`] with the given path, line, and column.
    #[must_use]
    pub const fn new(path: &'static str, line: u32, column: u32) -> Self {
        Self { path, line, column }
    }

    /// Returns the file path.
    #[must_use]
    pub const fn path(&self) -> &'static str {
        self.path
    }

    /// Returns the line number.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// Returns the column number.
    #[must_use]
    pub const fn column(&self) -> u32 {
        self.column
    }

    /// Returns the filename from the path.
    #[must_use]
    pub fn filename(&self) -> &str {
        // Try to find the last component from either path separator
        let unix_parts: Vec<&str> = self.path.split('/').collect();
        let windows_parts: Vec<&str> = self.path.split('\\').collect();
        
        // Use whichever gave us more parts (meaning it found separators)
        if unix_parts.len() > windows_parts.len() {
            unix_parts.last().copied().unwrap_or(self.path)
        } else {
            windows_parts.last().copied().unwrap_or(self.path)
        }
    }

    /// Returns a short representation of the location (filename:line:column).
    #[must_use]
    pub fn short(&self) -> String {
        format!("{}:{}:{}", self.filename(), self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn location_new_creates_location() {
        let location = Location::new("src/test.rs", 42, 10);
        assert_eq!(location.path, "src/test.rs");
        assert_eq!(location.line, 42);
        assert_eq!(location.column, 10);
    }

    #[test]
    fn location_accessors_work() {
        let location = Location::new("src/test.rs", 42, 10);
        assert_eq!(location.path(), "src/test.rs");
        assert_eq!(location.line(), 42);
        assert_eq!(location.column(), 10);
    }

    #[test]
    fn location_filename_returns_filename_from_unix_path() {
        let location = Location::new("src/step/test.rs", 1, 1);
        assert_eq!(location.filename(), "test.rs");
    }

    #[test]
    fn location_filename_returns_filename_from_windows_path() {
        let location = Location::new("src\\step\\test.rs", 1, 1);
        assert_eq!(location.filename(), "test.rs");
    }

    #[test]
    fn location_filename_returns_path_if_no_separators() {
        let location = Location::new("test.rs", 1, 1);
        assert_eq!(location.filename(), "test.rs");
    }

    #[test]
    fn location_short_returns_short_representation() {
        let location = Location::new("src/step/test.rs", 42, 10);
        assert_eq!(location.short(), "test.rs:42:10");
    }

    #[test]
    fn location_display_works() {
        let location = Location::new("src/test.rs", 42, 10);
        let display_output = format!("{}", location);
        assert_eq!(display_output, "src/test.rs:42:10");
    }

    #[test]
    fn location_debug_works() {
        let location = Location::new("src/test.rs", 42, 10);
        let debug_output = format!("{:?}", location);
        assert!(debug_output.contains("Location"));
        assert!(debug_output.contains("src/test.rs"));
        assert!(debug_output.contains("42"));
        assert!(debug_output.contains("10"));
    }

    #[test]
    fn location_equality_works() {
        let location1 = Location::new("src/test.rs", 42, 10);
        let location2 = Location::new("src/test.rs", 42, 10);
        let location3 = Location::new("src/other.rs", 42, 10);
        let location4 = Location::new("src/test.rs", 43, 10);
        let location5 = Location::new("src/test.rs", 42, 11);

        assert_eq!(location1, location2);
        assert_ne!(location1, location3);
        assert_ne!(location1, location4);
        assert_ne!(location1, location5);
    }

    #[test]
    fn location_ordering_works() {
        let location1 = Location::new("a.rs", 1, 1);
        let location2 = Location::new("b.rs", 1, 1);
        let location3 = Location::new("a.rs", 2, 1);
        let location4 = Location::new("a.rs", 1, 2);

        assert!(location1 < location2);  // Different files
        assert!(location1 < location3);  // Same file, different line
        assert!(location1 < location4);  // Same file, same line, different column
    }

    #[test]
    fn location_can_be_used_in_hashmap() {
        let location1 = Location::new("src/test1.rs", 10, 5);
        let location2 = Location::new("src/test2.rs", 20, 15);

        let mut map = HashMap::new();
        map.insert(location1, "step1");
        map.insert(location2, "step2");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&location1), Some(&"step1"));
        assert_eq!(map.get(&location2), Some(&"step2"));
    }

    #[test]
    fn location_can_be_used_in_hashset() {
        let location1 = Location::new("src/test.rs", 10, 5);
        let location2 = Location::new("src/test.rs", 20, 15);
        let location1_duplicate = Location::new("src/test.rs", 10, 5);

        let mut set = HashSet::new();
        set.insert(location1);
        set.insert(location2);
        set.insert(location1_duplicate);

        // Should only contain 2 unique locations
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn location_clone_works() {
        let location = Location::new("src/test.rs", 42, 10);
        let cloned = location.clone();

        assert_eq!(location, cloned);
        assert_eq!(location.path, cloned.path);
        assert_eq!(location.line, cloned.line);
        assert_eq!(location.column, cloned.column);
    }

    #[test]
    fn location_copy_works() {
        let location = Location::new("src/test.rs", 42, 10);
        let copied = location;

        assert_eq!(location, copied);
        assert_eq!(location.path, copied.path);
        assert_eq!(location.line, copied.line);
        assert_eq!(location.column, copied.column);
    }

    #[test]
    fn location_is_const_constructible() {
        const LOCATION: Location = Location::new("src/test.rs", 42, 10);
        assert_eq!(LOCATION.path(), "src/test.rs");
        assert_eq!(LOCATION.line(), 42);
        assert_eq!(LOCATION.column(), 10);
    }

    #[test]
    fn location_partial_ord_works() {
        let location1 = Location::new("a.rs", 1, 1);
        let location2 = Location::new("b.rs", 1, 1);

        assert_eq!(location1.partial_cmp(&location2), Some(std::cmp::Ordering::Less));
        assert_eq!(location2.partial_cmp(&location1), Some(std::cmp::Ordering::Greater));
        assert_eq!(location1.partial_cmp(&location1), Some(std::cmp::Ordering::Equal));
    }
}