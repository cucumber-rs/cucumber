#![allow(unused)]

use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while, take_while1},
    character::complete::{digit1, one_of},
    combinator::{map, verify},
    multi::{many0, many1, separated_list1},
    sequence::{delimited, preceded, tuple},
    IResult,
};
use nom_locate::LocatedSpan;

type Span<'s> = LocatedSpan<&'s str>;

#[derive(Debug)]
struct Parameter<'s>(Span<'s>);

#[derive(Debug)]
enum Option<'s> {
    Optional(Optional<'s>),
    // Parameter(Parameter<'s>),
    Text(Span<'s>),
}

#[derive(Debug)]
struct Optional<'s>(Vec<Option<'s>>);

#[derive(Debug)]
enum Alternative<'s> {
    Optional(Optional<'s>),
    Text(Span<'s>),
}

#[derive(Debug)]
struct Alternation<'s>(Vec<Vec<Alternative<'s>>>);

#[derive(Debug)]
enum SingleExpr<'s> {
    Alternation(Alternation<'s>),
    Optional(Optional<'s>),
    Parameter(Parameter<'s>),
    Text(Span<'s>),
    Space,
}

#[derive(Debug)]
struct Expr<'s>(Vec<SingleExpr<'s>>);

const SPECIAL_CHARS: &str = r#"{}()\/ "#;

fn or_space(f: impl Fn(char) -> bool) -> impl Fn(char) -> bool {
    move |c| c == ' ' || f(c)
}

fn is_text(c: char) -> bool {
    !SPECIAL_CHARS.contains(c) || matches!(c, '}' | ')')
}

fn is_name(c: char) -> bool {
    !SPECIAL_CHARS.contains(c) || c == ' '
}

fn is_right_boundary(c: char) -> bool {
    matches!(c, ' ' | '{' | '$')
}

fn is_left_boundary(c: char) -> bool {
    matches!(c, ' ' | '}' | '^')
}

fn parameter(input: Span) -> IResult<Span, Parameter> {
    map(
        delimited(
            tag("{"),
            escaped(take_while(is_name), '\\', one_of(SPECIAL_CHARS)),
            tag("}"),
        ),
        Parameter,
    )(input)
}

fn option(input: Span) -> IResult<Span, Option> {
    alt((
        map(optional, Option::Optional),
        map(
            verify(
                escaped(
                    take_while1(or_space(|c| is_text(c) && c != ')')),
                    '\\',
                    one_of(SPECIAL_CHARS),
                ),
                |s: &Span| !s.is_empty(),
            ),
            Option::Text,
        ),
    ))(input)
}

fn optional(input: Span) -> IResult<Span, Optional> {
    delimited(tag("("), map(many1(option), Optional), tag(")"))(input)
}

fn alternative(input: Span) -> IResult<Span, Alternative> {
    alt((
        map(optional, Alternative::Optional),
        map(
            verify(
                escaped(take_while1(is_text), '\\', one_of(SPECIAL_CHARS)),
                |s: &Span| !s.is_empty(),
            ),
            Alternative::Text,
        ),
    ))(input)
}

fn alternation(input: Span) -> IResult<Span, Alternation> {
    map(
        verify(
            separated_list1(tag("/"), many1(alternative)),
            |v: &[_]| v.len() > 1,
        ),
        Alternation,
    )(input)
}

fn single_expr(input: Span) -> IResult<Span, SingleExpr> {
    alt((
        map(alternation, SingleExpr::Alternation),
        map(optional, SingleExpr::Optional),
        map(parameter, SingleExpr::Parameter),
        map(
            verify(
                escaped(take_while1(is_text), '\\', one_of(SPECIAL_CHARS)),
                |s: &Span| !s.is_empty(),
            ),
            SingleExpr::Text,
        ),
        map(tag(" "), |_| SingleExpr::Space),
    ))(input)
}

fn expr(input: Span) -> IResult<Span, Expr> {
    map(many0(single_expr), Expr)(input)
}

#[cfg(test)]
mod spec {
    use super::{expr, Span};

    #[test]
    fn par() {
        let res = expr(Span::new(r"three () mice"));
        dbg!(res);
    }
}

// errors
// - empty alternation
// - alternation that contains only optional
// - optional that contain only optional (???)
// - optional that contain parameters
// - escaped non-reserved char
