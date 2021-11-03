#![allow(unused)]

mod combinators;

use std::iter;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::one_of,
    combinator::{map, peek, verify},
    error::{ErrorKind, ParseError},
    multi::{many0, many1, separated_list0},
    sequence::tuple,
    Err, IResult, Parser,
};
use nom_locate::LocatedSpan;

use self::combinators::{and_then, escaped0, map_err};

type Span<'s> = LocatedSpan<&'s str>;

const SPECIAL_CHARS: &str = r#"{}()\/ "#;

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
    fn first_span(&self) -> Span<'s> {
        if let Some(f) = self.0.first() {
            match f {
                Option::Optional(opt) => opt.first_span(),
                Option::Text(text) => *text,
            }
        } else {
            panic!("");
        }
    }
}

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

impl<'s> Alternation<'s> {
    fn contains_only_optional(&self) -> std::option::Option<Error<'s>> {
        for alt in &self.0 {
            if alt.len() == 1 {
                if let Some(Alternative::Optional(opt)) = alt.last() {
                    return Some(Error::OnlyOptionalInAlternation(
                        opt.first_span(),
                    ));
                }
            }
        }
        None
    }
}

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

fn escaped_special_chars0<'a, F, O1>(
    normal: F,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, Span<'a>, Error<'a>>
where
    F: nom::Parser<Span<'a>, O1, Error<'a>>,
{
    map_err(escaped0(normal, '\\', one_of(SPECIAL_CHARS)), |e| {
        if let Err::Error(Error::Other(span, ErrorKind::Escaped)) = e {
            Error::EscapedNonReservedCharacter(span).failure()
        } else {
            e
        }
    })
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

fn parameter(input: Span) -> IResult<Span, Parameter, Error> {
    let (input, opening_brace) = tag("{")(input)?;
    let (input, par_name) = escaped_special_chars0(take_while(is_name))(input)?;
    let (input, _) = map_err(tag("}"), |_| match input.chars().next() {
        Some('{')
            if peek(tuple((
                parameter,
                escaped_special_chars0(take_while(is_name)),
                tag("}"),
            )))(input)
            .is_ok() =>
        {
            Error::NestedParameter(input).failure()
        }
        Some('(') if peek(optional)(input).is_ok() => {
            Error::OptionalInParameter(input).failure()
        }
        Some(c) if SPECIAL_CHARS.contains(c) => {
            Error::UnescapedReservedCharacter(input).failure()
        }
        _ => Error::UnfinishedParameter(opening_brace).failure(),
    })(input)?;

    Ok((input, Parameter(par_name)))
}

fn option(input: Span) -> IResult<Span, Option, Error> {
    alt((
        map(optional, Option::Optional),
        map(
            verify(
                escaped_special_chars0(take_while(or_space(|c| {
                    is_text(c) && c != ')'
                }))),
                |s: &Span| !s.is_empty(),
            ),
            Option::Text,
        ),
    ))(input)
}

fn optional(input: Span) -> IResult<Span, Optional, Error> {
    let (input, opening_paren) = tag("(")(input)?;
    let (input, optional) = and_then(
        map_err(map(many1(option), Optional), |err| {
            if let Err::Error(Error::Other(span, ErrorKind::Many1)) = err {
                match span.chars().next() {
                    Some('{') if peek(parameter)(span).is_ok() => {
                        Error::ParameterInOptional(span)
                    }
                    Some('/') => Error::AlternationInOptional(span),
                    Some(')') => Error::EmptyOptional(span),
                    Some(c) if SPECIAL_CHARS.contains(c) => {
                        Error::UnescapedReservedCharacter(span)
                    }
                    _ => Error::EmptyOptional(span),
                }
                .failure()
            } else {
                err
            }
        }),
        |opt| {
            opt.can_be_simplified().map_or(Ok(opt), |sp| {
                Err(Err::Failure(Error::OptionCanBeSimplified(sp)))
            })
        },
    )(input)?;
    let (input, _) = map_err(tag(")"), |_| {
        Error::UnfinishedOptional(opening_paren).failure()
    })(input)?;

    Ok((input, optional))
}

fn alternative(input: Span) -> IResult<Span, Alternative, Error> {
    alt((
        map(optional, Alternative::Optional),
        map(
            escaped_special_chars0(take_while(is_text)),
            Alternative::Text,
        ),
    ))(input)
}

fn alternation(input: Span) -> IResult<Span, Alternation, Error> {
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
        let alt = Alternation(
            iter::once(iter::once(head).chain(head_rest).collect())
                .chain(tail)
                .collect(),
        );
        alt.contains_only_optional()
            .map_or(Ok((rest, alt)), |e| Err(e.failure()))
    } else {
        Err(Err::Failure(Error::EmptyAlternation(rest)))
    }
}

fn single_expr(input: Span) -> IResult<Span, SingleExpr, Error> {
    alt((
        map(alternation, SingleExpr::Alternation),
        map(optional, SingleExpr::Optional),
        map(parameter, SingleExpr::Parameter),
        map(
            verify(escaped_special_chars0(take_while(is_text)), |s: &Span| {
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
    NestedParameter(Span<'a>),
    OptionalInParameter(Span<'a>),
    EmptyAlternation(Span<'a>),
    EmptyOptional(Span<'a>),
    ParameterInOptional(Span<'a>),
    AlternationInOptional(Span<'a>),
    OptionCanBeSimplified(Span<'a>),
    UnfinishedParameter(Span<'a>),
    UnfinishedOptional(Span<'a>),
    UnescapedReservedCharacter(Span<'a>),
    EscapedNonReservedCharacter(Span<'a>),
    OnlyOptionalInAlternation(Span<'a>),
    Other(Span<'a>, ErrorKind),
}

impl<'a> Error<'a> {
    fn failure(self) -> Err<Self> {
        Err::Failure(self)
    }
}

impl<'a> ParseError<Span<'a>> for Error<'a> {
    fn from_error_kind(input: Span<'a>, kind: ErrorKind) -> Self {
        Self::Other(input, kind)
    }

    fn append(input: Span<'a>, kind: ErrorKind, other: Self) -> Self {
        if let Self::Other(..) = other {
            Self::from_error_kind(input, kind)
        } else {
            other
        }
    }
}

#[cfg(test)]
mod spec {
    use super::*;

    #[test]
    fn par() {
        let res = expr(Span::new(r"s/(s)"));
        dbg!(res);
    }
}

// errors
// - [x] empty alternation
// - [x] alternation that contains only optional
// - [x] alternation inside of optional
// - [x] optional can be simplified
// - [x] optional that contain parameters
// - [x] empty optional
// - [x] escaped non-reserved char

// to solve
// - [x] "({int})" returns wrong error
// - [x] figure out how to error in cases of
//   - [x] special chars in parameter: {(string)}
//   - [x] unbalanced parens: three (exceptionally\) {string\} mice

// spec and test-data difference
// - matching/does-not-allow-nested-optional
//   parser/optional-containing-nested-optional.
//   So which is it?
