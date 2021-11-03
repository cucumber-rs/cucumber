#![allow(unused)]

use std::{iter, ops::RangeFrom};

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

use nom::{
    AsChar, InputIter, InputLength, InputTake, InputTakeAtPosition, Offset,
    Slice,
};

/// Doesn't return if normal parser didn't consume anything.
fn escaped0<'a, Input: 'a, Error, F, G, O1, O2>(
    mut normal: F,
    control_char: char,
    mut escapable: G,
) -> impl FnMut(Input) -> IResult<Input, Input, Error>
where
    Input: Clone
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition
        + Slice<RangeFrom<usize>>
        + InputIter,
    <Input as InputIter>::Item: AsChar,
    F: Parser<Input, O1, Error>,
    G: Parser<Input, O2, Error>,
    Error: ParseError<Input>,
{
    move |input: Input| {
        let mut i = input.clone();
        let mut consumed_nothing = false;

        while i.input_len() > 0 {
            let current_len = i.input_len();

            match (normal.parse(i.clone()), consumed_nothing) {
                (Ok((i2, _)), false) => {
                    // return if we consumed everything or if the normal parser
                    // does not consume anything
                    if i2.input_len() == 0 {
                        return Ok((input.slice(input.input_len()..), input));
                    } else if i2.input_len() == current_len {
                        consumed_nothing = true;
                        // let index = input.offset(&i2);
                        // return Ok(input.take_split(index));
                    }
                    i = i2;
                }
                (Ok(..), true) | (Err(Err::Error(_)), _) => {
                    // unwrap() should be safe here since index < $i.input_len()
                    if i.iter_elements().next().unwrap().as_char()
                        == control_char
                    {
                        let next = control_char.len_utf8();
                        if next >= i.input_len() {
                            return Err(Err::Error(Error::from_error_kind(
                                input,
                                ErrorKind::Escaped,
                            )));
                        }
                        match escapable.parse(i.slice(next..)) {
                            Ok((i2, _)) => {
                                if i2.input_len() == 0 {
                                    return Ok((
                                        input.slice(input.input_len()..),
                                        input,
                                    ));
                                }
                                consumed_nothing = false;
                                i = i2;
                            }
                            Err(e) => {
                                return Err(Err::Error(
                                    Error::from_error_kind(
                                        i.slice(next..),
                                        ErrorKind::Escaped,
                                    ),
                                ));
                            }
                        }
                    } else {
                        let index = input.offset(&i);
                        return Ok(input.take_split(index));
                    }
                }
                (Err(e), _) => {
                    return Err(e);
                }
            }
        }

        Ok((input.slice(input.input_len()..), input))
    }
}

fn escaped_special_chars0<'a, F, O1>(
    normal: F,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, Span<'a>, Error<'a>>
where
    F: nom::Parser<Span<'a>, O1, Error<'a>>,
{
    map_err(escaped0(normal, '\\', one_of(SPECIAL_CHARS)), |e| {
        if let Err::Error(Error::Nom(span, ErrorKind::Escaped)) = e {
            Error::EscapedNonReservedCharacter(span).failure()
        } else {
            e
        }
    })
}

fn and_then<I, O1, O2, E: ParseError<I>, F, H>(
    mut parser: F,
    map_ok: H,
) -> impl FnMut(I) -> IResult<I, O2, E>
where
    F: Parser<I, O1, E>,
    H: Fn(O1) -> Result<O2, Err<E>>,
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
    G: FnOnce(Err<E>) -> Err<E> + Copy,
{
    move |input: I| parser.parse(input).map_err(map_err)
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
            if let Err::Error(Error::Nom(span, ErrorKind::Many1)) = err {
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
    // TODO: error on (s)/s (while s(s)/s is correct)

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
    Nom(Span<'a>, ErrorKind),
}

impl<'a> Error<'a> {
    fn failure(self) -> Err<Self> {
        Err::Failure(self)
    }
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
    use super::*;

    #[test]
    fn par() {
        let res = expr(Span::new(r"(\\n)"));
        dbg!(res);
    }
}

// errors
// - [x] empty alternation
// - [ ] alternation that contains only optional
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
