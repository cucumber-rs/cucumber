// Copyright (c) 2020-2026  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! ### Tag expressions
//!
//! You can read about tag expressions in the [Cucumber documentation](https://cucumber.io/docs/cucumber/api#tag-expressions).
//!
//! This implements the parsing apparatus for these expressions so that other crates like [`cucumber`](https://github.com/cucumber-rs/cucumber)
//! may take advantage of them.
//!
//! #### Usage
//!
//! ```
//! use gherkin::tagexpr::TagOperation;
//! # fn main() -> Result<(), peg::error::ParseError<peg::str::LineCol>> {
//! let op: TagOperation = "@a and @b".parse()?;
//! # Ok(())
//! # }
//! ```

use std::str::FromStr;

impl FromStr for TagOperation {
    type Err = peg::error::ParseError<peg::str::LineCol>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parser::gherkin_parser::tag_operation(s, &Default::default(), s)
    }
}

/// A parsed tree of operations for Gherkin tags.
#[derive(Debug, Clone)]
pub enum TagOperation {
    And(Box<TagOperation>, Box<TagOperation>),
    Or(Box<TagOperation>, Box<TagOperation>),
    Not(Box<TagOperation>),
    Tag(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_expr1() {
        let foo: TagOperation = "@foo and @bar".parse().unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(format!("{foo:?}"), "And(Tag(\"foo\"), Tag(\"bar\"))");
    }
    #[test]
    fn parse_tag_expr2() {
        let foo: TagOperation = "@foo or @bar".parse().unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(format!("{foo:?}"), "Or(Tag(\"foo\"), Tag(\"bar\"))");
    }

    #[test]
    fn parse_tag_expr1b() {
        let foo: TagOperation = "(@foo and @bar)"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(format!("{foo:?}"), "And(Tag(\"foo\"), Tag(\"bar\"))");
    }
    #[test]
    fn parse_tag_expr2b() {
        let foo: TagOperation = "(@foo or @bar)".parse().unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(format!("{foo:?}"), "Or(Tag(\"foo\"), Tag(\"bar\"))");
    }

    #[test]
    fn parse_tag_expr3() {
        let foo: TagOperation = "not @fat".parse().unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(format!("{foo:?}"), "Not(Tag(\"fat\"))");
    }

    #[test]
    fn parse_tag_expr4() {
        let foo: Result<TagOperation, _> = "@foo not @bar".parse();
        assert!(foo.is_err());
    }

    #[test]
    fn parse_tag_expr5() {
        let foo: TagOperation = "(not @foo) and not (@haha or @bar)"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(
            format!("{foo:?}"),
            "And(Not(Tag(\"foo\")), Not(Or(Tag(\"haha\"), Tag(\"bar\"))))",
        );
    }

    #[test]
    fn parse_tag_expr6() {
        let foo: TagOperation = "not @foo and not @haha or @bar"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(
            format!("{foo:?}"),
            "Or(And(Not(Tag(\"foo\")), Not(Tag(\"haha\"))), Tag(\"bar\"))",
        );
    }

    #[test]
    fn parse_tag_expr7() {
        let foo: TagOperation = "not (@a or @b) and (@c or not @d)"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(
            format!("{foo:?}"),
            "And(Not(Or(Tag(\"a\"), Tag(\"b\"))), Or(Tag(\"c\"), Not(Tag(\"d\"))))",
        );
    }

    #[test]
    fn parse_tag_expr8() {
        let foo: TagOperation = "@a or @b and @c or not @d"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(
            format!("{foo:?}"),
            "Or(Or(Tag(\"a\"), And(Tag(\"b\"), Tag(\"c\"))), Not(Tag(\"d\")))",
        );
    }

    #[test]
    fn parse_tag_expr9() {
        let foo: TagOperation = "@bar\\\\\\)\\ \\("
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(format!("{foo:?}"), "Tag(\"bar\\\\) (\")");
    }

    #[test]
    fn parse_tag_expr10() {
        let foo: TagOperation = "(@foo and @bar\\))"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
    }

    #[test]
    fn parse_tag_expr11() {
        let foo: TagOperation = "not (@\\)a or @\\(b) and (@c or not @d)"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);
        assert_eq!(
            format!("{foo:?}"),
            "And(Not(Or(Tag(\")a\"), Tag(\"(b\"))), Or(Tag(\"c\"), Not(Tag(\"d\"))))",
        );
    }

    #[test]
    fn parse_tag_expr12() {
        let err = "@bar\\".parse::<TagOperation>().unwrap_err();
        println!("{:#?}", err);
    }

    #[test]
    fn parse_tag_expr13() {
        let foo: TagOperation = "not @a or @b and not @c or not @d or @e and @f"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo);

        let foo2: TagOperation = "( ( ( not ( @a ) or ( @b and not ( @c ) ) ) \
                                  or not ( @d ) ) or ( @e and @f ) )"
            .parse()
            .unwrap_or_else(|e| panic!("{}", e));
        println!("{:#?}", foo2);

        assert_eq!(format!("{foo:?}"), format!("{foo2:?}"));
    }
}
