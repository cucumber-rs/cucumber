//! Regex utilities for step matching.
//!
//! This module provides the [`HashableRegex`] wrapper that implements
//! [`Eq`], [`Ord`], and [`Hash`] traits for [`Regex`] objects, enabling
//! their use in hash maps and other collections.

use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use derive_more::with_trait::{Debug, Deref, DerefMut, Display};
use regex::Regex;

/// [`Regex`] wrapper implementing [`Eq`], [`Ord`] and [`Hash`].
#[derive(Clone, Debug, Deref, DerefMut, Display)]
pub struct HashableRegex(Regex);

impl HashableRegex {
    /// Creates a new [`HashableRegex`] from a [`Regex`].
    #[must_use]
    pub fn new(regex: Regex) -> Self {
        Self(regex)
    }

    /// Returns a reference to the inner [`Regex`].
    #[must_use]
    pub fn inner(&self) -> &Regex {
        &self.0
    }

    /// Consumes the wrapper and returns the inner [`Regex`].
    #[must_use]
    pub fn into_inner(self) -> Regex {
        self.0
    }

    /// Returns the regex pattern as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn hashable_regex_new_creates_wrapper() {
        let regex = Regex::new(r"test").unwrap();
        let hashable = HashableRegex::new(regex.clone());
        assert_eq!(hashable.as_str(), regex.as_str());
    }

    #[test]
    fn hashable_regex_from_regex_works() {
        let regex = Regex::new(r"test").unwrap();
        let hashable: HashableRegex = regex.clone().into();
        assert_eq!(hashable.as_str(), regex.as_str());
    }

    #[test]
    fn hashable_regex_inner_returns_regex_reference() {
        let regex = Regex::new(r"test").unwrap();
        let hashable = HashableRegex::new(regex.clone());
        assert_eq!(hashable.inner().as_str(), regex.as_str());
    }

    #[test]
    fn hashable_regex_into_inner_returns_regex() {
        let regex = Regex::new(r"test").unwrap();
        let pattern = regex.as_str().to_string();
        let hashable = HashableRegex::new(regex);
        let inner = hashable.into_inner();
        assert_eq!(inner.as_str(), pattern);
    }

    #[test]
    fn hashable_regex_as_str_returns_pattern() {
        let regex = Regex::new(r"test pattern").unwrap();
        let hashable = HashableRegex::new(regex);
        assert_eq!(hashable.as_str(), "test pattern");
    }

    #[test]
    fn hashable_regex_equality_works() {
        let regex1 = Regex::new(r"test").unwrap();
        let regex2 = Regex::new(r"test").unwrap();
        let regex3 = Regex::new(r"different").unwrap();
        
        let hashable1 = HashableRegex::new(regex1);
        let hashable2 = HashableRegex::new(regex2);
        let hashable3 = HashableRegex::new(regex3);
        
        assert_eq!(hashable1, hashable2);
        assert_ne!(hashable1, hashable3);
    }

    #[test]
    fn hashable_regex_ordering_works() {
        let regex_a = Regex::new(r"a").unwrap();
        let regex_b = Regex::new(r"b").unwrap();
        let regex_z = Regex::new(r"z").unwrap();
        
        let hashable_a = HashableRegex::new(regex_a);
        let hashable_b = HashableRegex::new(regex_b);
        let hashable_z = HashableRegex::new(regex_z);
        
        assert!(hashable_a < hashable_b);
        assert!(hashable_b < hashable_z);
        assert!(hashable_a < hashable_z);
        
        assert_eq!(hashable_a.cmp(&hashable_b), Ordering::Less);
        assert_eq!(hashable_b.cmp(&hashable_a), Ordering::Greater);
        assert_eq!(hashable_a.cmp(&hashable_a), Ordering::Equal);
    }

    #[test]
    fn hashable_regex_can_be_used_in_hashmap() {
        let regex1 = Regex::new(r"pattern1").unwrap();
        let regex2 = Regex::new(r"pattern2").unwrap();
        
        let mut map = HashMap::new();
        map.insert(HashableRegex::new(regex1), "value1");
        map.insert(HashableRegex::new(regex2), "value2");
        
        assert_eq!(map.len(), 2);
        
        let lookup_regex = Regex::new(r"pattern1").unwrap();
        let lookup_key = HashableRegex::new(lookup_regex);
        assert_eq!(map.get(&lookup_key), Some(&"value1"));
    }

    #[test]
    fn hashable_regex_can_be_used_in_hashset() {
        let regex1 = Regex::new(r"pattern1").unwrap();
        let regex2 = Regex::new(r"pattern2").unwrap();
        let regex1_duplicate = Regex::new(r"pattern1").unwrap();
        
        let mut set = HashSet::new();
        set.insert(HashableRegex::new(regex1));
        set.insert(HashableRegex::new(regex2));
        set.insert(HashableRegex::new(regex1_duplicate));
        
        // Should only contain 2 unique patterns
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn hashable_regex_hash_consistency() {
        let regex1 = Regex::new(r"test").unwrap();
        let regex2 = Regex::new(r"test").unwrap();
        
        let hashable1 = HashableRegex::new(regex1);
        let hashable2 = HashableRegex::new(regex2);
        
        // Equal objects must have equal hashes
        assert_eq!(hashable1, hashable2);
        
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        
        hashable1.hash(&mut hasher1);
        hashable2.hash(&mut hasher2);
        
        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn hashable_regex_deref_works() {
        let regex = Regex::new(r"test (\d+)").unwrap();
        let hashable = HashableRegex::new(regex);
        
        // Should be able to call Regex methods directly
        assert!(hashable.is_match("test 123"));
        assert!(!hashable.is_match("no match"));
        
        let captures = hashable.captures("test 456").unwrap();
        assert_eq!(captures.get(1).unwrap().as_str(), "456");
    }

    #[test]
    fn hashable_regex_clone_works() {
        let regex = Regex::new(r"test").unwrap();
        let hashable = HashableRegex::new(regex);
        let cloned = hashable.clone();
        
        assert_eq!(hashable, cloned);
        assert_eq!(hashable.as_str(), cloned.as_str());
    }

    #[test]
    fn hashable_regex_display_works() {
        let regex = Regex::new(r"test pattern").unwrap();
        let hashable = HashableRegex::new(regex);
        let display_output = format!("{}", hashable);
        assert_eq!(display_output, "test pattern");
    }

    #[test]
    fn hashable_regex_debug_works() {
        let regex = Regex::new(r"test").unwrap();
        let hashable = HashableRegex::new(regex);
        let debug_output = format!("{:?}", hashable);
        assert!(debug_output.contains("HashableRegex"));
    }
}