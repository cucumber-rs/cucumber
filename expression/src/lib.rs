#![allow(unused)]

use std::iter;

use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_while, take_while1},
    character::complete::{digit1, one_of},
    combinator::{cut, flat_map, map, verify},
    error::{ErrorKind, ParseError},
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, preceded, tuple},
    IResult, Parser,
};
use nom_locate::LocatedSpan;

type Span<'s> = LocatedSpan<&'s str>;

#[derive(Debug)]
struct Parameter<'s>(Span<'s>);

#[derive(Debug)]
enum Option<'s> {
    Optional(Optional<'s>),
    Text(Span<'s>),
}

#[derive(Debug)]
struct Optional<'s>(Vec<Option<'s>>);

impl<'s> Optional<'s> {
    fn can_be_simplified(&self) -> std::option::Option<Span<'s>> {
        if self.0.len() == 1 {
            if let Some(Option::Optional(nested_opt)) = self.0.last() {
                if nested_opt.0.len() == 1 {
                    if let Some(Option::Text(nested_span)) = nested_opt.0.last()
                    {
                        return Some(*nested_span);
                    }
                }
            }
        }
        None
    }
}

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

fn escaped_special_chars<'a, Error, F, O1>(
    mut normal: F,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, Span<'a>, Error>
where
    F: nom::Parser<Span<'a>, O1, Error>,
    Error: nom::error::ParseError<Span<'a>>,
{
    escaped(normal, '\\', one_of(SPECIAL_CHARS))
}

fn and_then<I, O1, O2, E: ParseError<I>, F, H>(
    mut parser: F,
    map_ok: H,
) -> impl FnMut(I) -> IResult<I, O2, E>
where
    F: Parser<I, O1, E>,
    H: Fn(O1) -> Result<O2, nom::Err<E>>,
{
    move |input: I| {
        parser
            .parse(input)
            .and_then(|(rest, parsed)| map_ok(parsed).map(|ok| (rest, ok)))
    }
}

fn map_err<I, O1, E: ParseError<I>, F, G>(
    mut parser: F,
    map_err: G,
) -> impl FnMut(I) -> IResult<I, O1, E>
where
    F: Parser<I, O1, E>,
    G: FnOnce(nom::Err<E>) -> nom::Err<E> + Copy,
{
    move |input: I| parser.parse(input).map_err(map_err)
}

fn map_res<I, O1, O2, E: ParseError<I>, F, G, H>(
    mut parser: F,
    map_ok: H,
    map_err: G,
) -> impl FnMut(I) -> IResult<I, O2, E>
where
    F: Parser<I, O1, E>,
    G: Fn(nom::Err<E>) -> nom::Err<E>,
    H: Fn(O1) -> Result<O2, nom::Err<E>>,
{
    move |input: I| match parser.parse(input) {
        Ok((rest, parsed)) => map_ok(parsed).map(|ok| (rest, ok)),
        Err(e) => Err(map_err(e)),
    }
}

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

fn parameter(input: Span) -> IResult<Span, Parameter, Error> {
    map(
        delimited(
            tag("{"),
            escaped_special_chars(take_while(is_name)),
            tag("}"),
        ),
        Parameter,
    )(input)
}

fn option<'s>(input: Span<'s>) -> IResult<Span<'s>, Option<'s>, Error<'s>> {
    use nom::Err::Failure;

    alt((
        map(optional, Option::Optional),
        map(
            flat_map(
                verify(
                    escaped_special_chars(take_while(or_space(|c| {
                        is_text(c) && c != ')'
                    }))),
                    |s: &Span| !s.is_empty(),
                ),
                |parsed: Span<'s>| {
                    move |rest: Span<'s>| {
                        if let Some('{') = rest.chars().next() {
                            Err(Failure(Error::ParameterInOptional(rest)))
                        } else {
                            Ok((rest, parsed))
                        }
                    }
                },
            ),
            Option::Text,
        ),
    ))(input)
}

fn optional(input: Span) -> IResult<Span, Optional, Error> {
    use nom::Err::Failure;

    delimited(
        tag("("),
        and_then(
            map_err(map(many1(option), Optional), |err| {
                if let nom::Err::Error(Error::Nom(span, ErrorKind::Many1)) = err
                {
                    Failure(Error::EmptyOptional(span))
                } else {
                    err
                }
            }),
            |opt| {
                opt.can_be_simplified().map_or(Ok(opt), |sp| {
                    Err(Failure(Error::OptionCanBeSimplified(sp)))
                })
            },
        ),
        tag(")"),
    )(input)
}

fn alternative(input: Span) -> IResult<Span, Alternative, Error> {
    alt((
        map(optional, Alternative::Optional),
        map(
            escaped_special_chars(take_while(is_text)),
            Alternative::Text,
        ),
    ))(input)
}

fn alternation(input: Span) -> IResult<Span, Alternation, Error> {
    use nom::Err::Failure;

    let not_empty = |alt: &Alternative| {
        if let Alternative::Text(text) = alt {
            !text.is_empty()
        } else {
            true
        }
    };

    let (rest, (head, head_rest, _, tail)) = tuple((
        alternative,
        many0(verify(alternative, not_empty)),
        tag("/"),
        separated_list0(tag("/"), many1(verify(alternative, not_empty))),
    ))(input)?;

    if not_empty(&head) && !tail.is_empty() {
        let alt = iter::once(iter::once(head).chain(head_rest).collect())
            .chain(tail)
            .collect();
        Ok((rest, Alternation(alt)))
    } else {
        Err(Failure(Error::EmptyAlternation(rest)))
    }
}

fn single_expr(input: Span) -> IResult<Span, SingleExpr, Error> {
    alt((
        map(alternation, SingleExpr::Alternation),
        map(optional, SingleExpr::Optional),
        map(parameter, SingleExpr::Parameter),
        map(
            verify(escaped_special_chars(take_while(is_text)), |s: &Span| {
                !s.is_empty()
            }),
            SingleExpr::Text,
        ),
        map(tag(" "), |_| SingleExpr::Space),
    ))(input)
}

fn expr(input: Span) -> IResult<Span, Expr, Error> {
    map(many0(single_expr), Expr)(input)
}

#[derive(Debug)]
enum Error<'a> {
    EmptyAlternation(Span<'a>),
    EmptyOptional(Span<'a>),
    ParameterInOptional(Span<'a>),
    OptionCanBeSimplified(Span<'a>),
    Nom(Span<'a>, ErrorKind),
}

impl<'a> ParseError<Span<'a>> for Error<'a> {
    fn from_error_kind(input: Span<'a>, kind: ErrorKind) -> Self {
        Self::Nom(input, kind)
    }

    fn append(input: Span<'a>, kind: ErrorKind, other: Self) -> Self {
        if let Self::Nom(..) = other {
            Self::from_error_kind(input, kind)
        } else {
            other
        }
    }
}

#[cfg(test)]
mod spec {
    use super::{alternation, expr, Span};

    #[test]
    fn par() {
        let res = expr(Span::new(r"three (exceptionally\) {string\} mice"));
        dbg!(res);
    }
}

// errors
// - [ ] empty alternation
// - [ ] alternation that contains only optional
// - [ ] alternation inside of optional
// - [x] optional can be simplified
// - [x] optional that contain parameters
// - [x] empty optional
// - [ ] escaped non-reserved char

// to solve
// - "({int})" returns wrong error
// - figure out how to error in cases of
//   - special chars in parameter: {(string)}
//   - unbalanced parens: three (exceptionally\) {string\} mice

// spec and test-data difference
// - matching/does-not-allow-alternation-with-empty-alternative-by-adjacent
//   "three (brown)/black mice" should be legal
// - matching/does-not-allow-nested-optional
//   parser/optional-containing-nested-optional.
//   So which is it?
