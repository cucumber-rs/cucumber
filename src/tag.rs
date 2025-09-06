// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Extension of a [`TagOperation`].

use gherkin::tagexpr::TagOperation;
use sealed::sealed;

/// Extension of a [`TagOperation`] allowing to evaluate it.
#[sealed]
pub trait Ext {
    /// Evaluates this [`TagOperation`] for the given `tags`.
    #[must_use]
    fn eval<I, S>(&self, tags: I) -> bool
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S> + Clone;
}

#[sealed]
impl Ext for TagOperation {
    fn eval<I, S>(&self, tags: I) -> bool
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S> + Clone,
    {
        match self {
            Self::And(l, r) => l.eval(tags.clone()) & r.eval(tags),
            Self::Or(l, r) => l.eval(tags.clone()) | r.eval(tags),
            Self::Not(t) => !t.eval(tags),
            Self::Tag(t) => tags.into_iter().any(|tag| tag.as_ref() == t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gherkin::tagexpr::TagOperation;

    #[test]
    fn test_tag_evaluation_single_tag() {
        let tag_op = TagOperation::Tag("smoke".to_string());
        
        // Should match when tag is present
        assert!(tag_op.eval(["smoke", "fast"]));
        assert!(tag_op.eval(vec!["smoke"]));
        
        // Should not match when tag is absent
        assert!(!tag_op.eval(["slow", "integration"]));
        assert!(!tag_op.eval(Vec::<String>::new()));
    }

    #[test]
    fn test_tag_evaluation_and_operation() {
        let tag_op = TagOperation::And(
            Box::new(TagOperation::Tag("smoke".to_string())),
            Box::new(TagOperation::Tag("fast".to_string())),
        );
        
        // Should match when both tags are present
        assert!(tag_op.eval(["smoke", "fast", "unit"]));
        
        // Should not match when only one tag is present
        assert!(!tag_op.eval(["smoke", "slow"]));
        assert!(!tag_op.eval(["fast", "integration"]));
        
        // Should not match when neither tag is present
        assert!(!tag_op.eval(["slow", "integration"]));
    }

    #[test]
    fn test_tag_evaluation_or_operation() {
        let tag_op = TagOperation::Or(
            Box::new(TagOperation::Tag("smoke".to_string())),
            Box::new(TagOperation::Tag("integration".to_string())),
        );
        
        // Should match when either tag is present
        assert!(tag_op.eval(["smoke", "fast"]));
        assert!(tag_op.eval(["integration", "slow"]));
        assert!(tag_op.eval(["smoke", "integration"]));
        
        // Should not match when neither tag is present
        assert!(!tag_op.eval(["unit", "fast"]));
    }

    #[test]
    fn test_tag_evaluation_not_operation() {
        let tag_op = TagOperation::Not(
            Box::new(TagOperation::Tag("slow".to_string())),
        );
        
        // Should match when tag is not present
        assert!(tag_op.eval(["smoke", "fast"]));
        assert!(tag_op.eval(Vec::<String>::new()));
        
        // Should not match when tag is present
        assert!(!tag_op.eval(["slow", "integration"]));
        assert!(!tag_op.eval(vec!["slow"]));
    }

    #[test]
    fn test_tag_evaluation_complex_expression() {
        // (@smoke or @integration) and not @slow
        let tag_op = TagOperation::And(
            Box::new(TagOperation::Or(
                Box::new(TagOperation::Tag("smoke".to_string())),
                Box::new(TagOperation::Tag("integration".to_string())),
            )),
            Box::new(TagOperation::Not(
                Box::new(TagOperation::Tag("slow".to_string())),
            )),
        );
        
        // Should match: has smoke and no slow
        assert!(tag_op.eval(["smoke", "fast"]));
        
        // Should match: has integration and no slow  
        assert!(tag_op.eval(["integration", "unit"]));
        
        // Should not match: has smoke but also slow
        assert!(!tag_op.eval(["smoke", "slow"]));
        
        // Should not match: has integration but also slow
        assert!(!tag_op.eval(["integration", "slow"]));
        
        // Should not match: no smoke or integration (even without slow)
        assert!(!tag_op.eval(["unit", "fast"]));
    }

    #[test]
    fn test_tag_evaluation_with_string_refs() {
        let tag_op = TagOperation::Tag("smoke".to_string());
        let tags = vec!["smoke", "fast"];
        
        // Test with &str references
        assert!(tag_op.eval(&tags));
        assert!(tag_op.eval(tags.iter()));
    }

    #[test]
    fn test_tag_evaluation_case_sensitive() {
        let tag_op = TagOperation::Tag("Smoke".to_string());
        
        // Should be case sensitive
        assert!(tag_op.eval(["Smoke"]));
        assert!(!tag_op.eval(["smoke"]));
        assert!(!tag_op.eval(["SMOKE"]));
    }

    #[test]
    fn test_tag_evaluation_empty_tags() {
        let tag_op = TagOperation::Tag("smoke".to_string());
        
        // Should not match empty tag list
        assert!(!tag_op.eval(Vec::<String>::new()));
        assert!(!tag_op.eval(std::iter::empty::<String>()));
    }

    #[test]
    fn test_tag_evaluation_whitespace_tags() {
        let tag_op = TagOperation::Tag(" smoke ".to_string());
        
        // Should match exact string including whitespace
        assert!(tag_op.eval([" smoke "]));
        assert!(!tag_op.eval(["smoke"]));
    }

    #[test]
    fn test_nested_and_or_operations() {
        // (@smoke and @fast) or (@integration and @slow)
        let tag_op = TagOperation::Or(
            Box::new(TagOperation::And(
                Box::new(TagOperation::Tag("smoke".to_string())),
                Box::new(TagOperation::Tag("fast".to_string())),
            )),
            Box::new(TagOperation::And(
                Box::new(TagOperation::Tag("integration".to_string())),
                Box::new(TagOperation::Tag("slow".to_string())),
            )),
        );
        
        // Should match: smoke and fast
        assert!(tag_op.eval(["smoke", "fast", "unit"]));
        
        // Should match: integration and slow
        assert!(tag_op.eval(["integration", "slow", "e2e"]));
        
        // Should not match: smoke without fast
        assert!(!tag_op.eval(["smoke", "slow"]));
        
        // Should not match: integration without slow
        assert!(!tag_op.eval(["integration", "fast"]));
        
        // Should not match: neither combination
        assert!(!tag_op.eval(["unit", "e2e"]));
    }

    #[test]
    fn test_multiple_not_operations() {
        // not @slow and not @flaky
        let tag_op = TagOperation::And(
            Box::new(TagOperation::Not(
                Box::new(TagOperation::Tag("slow".to_string())),
            )),
            Box::new(TagOperation::Not(
                Box::new(TagOperation::Tag("flaky".to_string())),
            )),
        );
        
        // Should match: neither slow nor flaky
        assert!(tag_op.eval(["smoke", "fast"]));
        assert!(tag_op.eval(Vec::<String>::new()));
        
        // Should not match: has slow
        assert!(!tag_op.eval(["slow", "smoke"]));
        
        // Should not match: has flaky
        assert!(!tag_op.eval(["flaky", "fast"]));
        
        // Should not match: has both
        assert!(!tag_op.eval(["slow", "flaky"]));
    }
}
