use std::hash::{Hash, Hasher};
use std::ops::Deref;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct HashableRegex(pub Regex);

impl std::fmt::Display for HashableRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for HashableRegex {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HashableRegex {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.as_str().cmp(other.0.as_str())
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
