use std::{iter, ops::RangeFrom};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::one_of,
    combinator::{map, peek, verify},
    error::{ErrorKind, ParseError},
    multi::{many0, many1, separated_list0},
    sequence::tuple,
    AsChar, Err, FindToken, IResult, InputIter, InputLength, InputTake,
    InputTakeAtPosition, Offset, Parser, Slice,
};

use crate::{
    ast::{
        Alternation, Alternative, Expression, Option, Optional, Parameter,
        SingleExpr, Spanned,
    },
    combinator::{and_then, escaped0, map_err},
};

/// Reserved characters that require special handling.
const RESERVED_CHARS: &str = r#"{}()\/ "#;

/// Matches `normal` and escaped with `\` [`RESERVED_CHARS`].
///
/// Uses [`escaped0`] under the hood.
///
/// # Errors
///
/// ## Recoverable [`Error`]
///
/// - If `normal` parser errors.
///
/// ## Irrecoverable [`Failure`]
///
/// - If `normal` parser fails.
/// - [`EscapedNonReservedCharacter`] if non-reserved character was escaped.
///
/// [`Error`]: Err::Error
/// [`EscapedNonReservedCharacter`]: Error::EscapedNonReservedCharacter
/// [`Failure`]: Err::Failure
fn escaped_reserved_chars0<'a, Input: 'a, F, O1>(
    normal: F,
) -> impl FnMut(Input) -> IResult<Input, Input, Error<'a>>
where
    Input: Clone
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition
        + Slice<RangeFrom<usize>>
        + InputIter,
    <Input as InputIter>::Item: AsChar + Copy,
    F: Parser<Input, O1, Error<'a>>,
    Error<'a>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    map_err(escaped0(normal, '\\', one_of(RESERVED_CHARS)), |e| {
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
    !RESERVED_CHARS.contains(c) || matches!(c, '}' | ')')
}

/// # Syntax
///
/// ```text
/// parameter := '{' name* '}'
/// name      := character except '{' | '}' | '(' | '/'
/// ```
///
/// Note: `{`, `}`, `(`, `/` still can be used if escaped with `\`
///
/// # Example
///
/// ```text
/// {}
/// {name}
/// {with spaces}
/// {escaped \/\{\(}
/// {no need to escape )}
/// ```
///
/// # Errors
///
/// ## Recoverable [`Error`]s
///
/// - If `input` doesn't start with `{`
///
/// ## Irrecoverable [`Failure`]s
///
/// - [`NestedParameter`]
/// - [`OptionalInParameter`]
/// - [`UnescapedReservedCharacter`]
/// - [`UnfinishedParameter`]
///
/// [`Error`]: Err::Error
/// [`Failure`]: Err::Failure
/// [`NestedParameter`]: Error::NestedParameter
/// [`OptionalInParameter`]: Error::OptionalInParameter
/// [`UnescapedReservedCharacter`]: Error::UnescapedReservedCharacter
/// [`UnfinishedParameter`]: Error::UnfinishedParameter
fn parameter<'s>(
    input: Spanned<'s>,
) -> IResult<Spanned<'s>, Parameter<'s>, Error<'s>> {
    let is_name = |c| !"{}(\\/".contains(c);

    let fail = |input: Spanned<'s>, opening_brace| {
        match input.chars().next() {
            Some('{') => {
                if let Ok((_, (par, ..))) = peek(tuple((
                    parameter,
                    escaped_reserved_chars0(take_while(is_name)),
                    tag("}"),
                )))(input)
                {
                    return Error::NestedParameter(input.take(par.0.len() + 2))
                        .failure();
                }
                return Error::UnescapedReservedCharacter(input.take(1)).failure();
            }
            Some('(') => {
                if let Ok((_, opt)) = peek(optional)(input) {
                    return Error::OptionalInParameter(input.take(opt.span_len()))
                        .failure();
                }
                return Error::UnescapedReservedCharacter(input.take(1)).failure();
            }
            Some(c) if RESERVED_CHARS.contains(c) => {
                return Error::UnescapedReservedCharacter(input.take(1)).failure();
            }
            _ => {}
        }
        Error::UnfinishedParameter(opening_brace).failure()
    };

    let (input, opening_brace) = tag("{")(input)?;
    let (input, par_name) =
        escaped_reserved_chars0(take_while(is_name))(input)?;
    let (input, _) = map_err(tag("}"), |_| fail(input, opening_brace))(input)?;

    Ok((input, Parameter(par_name)))
}

fn option(input: Spanned) -> IResult<Spanned, Option, Error> {
    alt((
        map(optional, Option::Optional),
        map(
            verify(
                escaped_reserved_chars0(take_while(or_space(|c| {
                    is_text(c) && c != ')'
                }))),
                |s: &Spanned| !s.is_empty(),
            ),
            Option::Text,
        ),
    ))(input)
}

fn optional(input: Spanned) -> IResult<Spanned, Optional, Error> {
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
                    Some(c) if RESERVED_CHARS.contains(c) => {
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

fn alternative(input: Spanned) -> IResult<Spanned, Alternative, Error> {
    alt((
        map(optional, Alternative::Optional),
        map(
            escaped_reserved_chars0(take_while(is_text)),
            Alternative::Text,
        ),
    ))(input)
}

fn alternation(input: Spanned) -> IResult<Spanned, Alternation, Error> {
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

fn single_expr(input: Spanned) -> IResult<Spanned, SingleExpr, Error> {
    alt((
        map(alternation, SingleExpr::Alternation),
        map(optional, SingleExpr::Optional),
        map(parameter, SingleExpr::Parameter),
        map(
            verify(
                escaped_reserved_chars0(take_while(is_text)),
                |s: &Spanned| !s.is_empty(),
            ),
            SingleExpr::Text,
        ),
        map(tag(" "), |_| SingleExpr::Space),
    ))(input)
}

fn expr(input: Spanned) -> IResult<Spanned, Expression, Error> {
    map(many0(single_expr), Expression)(input)
}

#[derive(Debug, Eq, PartialEq)]
pub enum Error<'a> {
    NestedParameter(Spanned<'a>),
    OptionalInParameter(Spanned<'a>),
    EmptyAlternation(Spanned<'a>),
    EmptyOptional(Spanned<'a>),
    ParameterInOptional(Spanned<'a>),
    AlternationInOptional(Spanned<'a>),
    OptionCanBeSimplified(Spanned<'a>),
    UnfinishedParameter(Spanned<'a>),
    UnfinishedOptional(Spanned<'a>),
    UnescapedReservedCharacter(Spanned<'a>),
    EscapedNonReservedCharacter(Spanned<'a>),
    OnlyOptionalInAlternation(Spanned<'a>),
    Other(Spanned<'a>, ErrorKind),
}

impl<'a> Error<'a> {
    fn failure(self) -> Err<Self> {
        Err::Failure(self)
    }
}

impl<'a> ParseError<Spanned<'a>> for Error<'a> {
    fn from_error_kind(input: Spanned<'a>, kind: ErrorKind) -> Self {
        Self::Other(input, kind)
    }

    fn append(input: Spanned<'a>, kind: ErrorKind, other: Self) -> Self {
        if let Self::Other(..) = other {
            Self::from_error_kind(input, kind)
        } else {
            other
        }
    }
}

#[cfg(test)]
mod spec {
    use super::{
        parameter, Err, Error, ErrorKind, IResult, Parameter, Spanned,
    };

    fn eq(left: impl AsRef<str>, right: impl AsRef<str>) {
        assert_eq!(
            left.as_ref().replace(' ', "").replace('\n', ""),
            right.as_ref().replace(' ', "").replace('\n', "")
        );
    }

    mod parameter {
        use super::{
            eq, parameter, Err, Error, ErrorKind, IResult, Parameter, Spanned,
        };

        fn unwrap_parameter<'s>(
            par: IResult<Spanned<'s>, Parameter<'s>, Error<'s>>,
        ) -> Parameter<'s> {
            let (rest, par) = par.expect("ok");
            assert_eq!(*rest, "");
            par
        }

        #[test]
        fn empty() {
            eq(
                format!(
                    "{:?}",
                    unwrap_parameter(parameter(Spanned::new("{}")))
                ),
                r#"Parameter (
                LocatedSpan {
                    offset: 1,
                    line: 1,
                    fragment: "",
                    extra: ()
                }
            )"#,
            );
        }

        #[test]
        fn named() {
            eq(
                format!(
                    "{:?}",
                    unwrap_parameter(parameter(Spanned::new("{string}")))
                ),
                r#"Parameter (
                LocatedSpan {
                    offset: 1,
                    line: 1,
                    fragment: "string",
                    extra: ()
                }
            )"#,
            );
        }

        #[test]
        fn named_with_spaces() {
            eq(
                format!(
                    "{:?}",
                    unwrap_parameter(parameter(Spanned::new("{with space}")))
                ),
                r#"Parameter (
                LocatedSpan {
                    offset: 1,
                    line: 1,
                    fragment: "with space",
                    extra: ()
                }
            )"#,
            );
        }

        #[test]
        fn named_with_escaped() {
            eq(
                format!(
                    "{:?}",
                    unwrap_parameter(parameter(Spanned::new("{with \\{}")))
                ),
                r#"Parameter (
                LocatedSpan {
                    offset: 1,
                    line: 1,
                    fragment: "with \\{",
                    extra: ()
                }
            )"#,
            );
        }

        #[test]
        fn named_with_closing_brace() {
            eq(
                format!(
                    "{:?}",
                    unwrap_parameter(parameter(Spanned::new("{with )}")))
                ),
                r#"Parameter (
                LocatedSpan {
                    offset: 1,
                    line: 1,
                    fragment: "with )",
                    extra: ()
                }
            )"#,
            );
        }

        /// - [`UnfinishedParameter`]
        #[test]
        fn errors_on_empty() {
            let span = Spanned::new("");

            assert_eq!(
                parameter(span),
                Err(Err::Error(Error::Other(span, ErrorKind::Tag))),
            );
        }

        #[test]
        fn fails_on_nested() {
            let err = [
                parameter(Spanned::new("{{nest}}")).expect_err("error"),
                parameter(Spanned::new("{before{nest}}")).expect_err("error"),
                parameter(Spanned::new("{{nest}after}")).expect_err("error"),
                parameter(Spanned::new("{bef{nest}aft}")).expect_err("error"),
            ];

            match err {
                [
                    Err::Failure(Error::NestedParameter(e1)),
                    Err::Failure(Error::NestedParameter(e2)),
                    Err::Failure(Error::NestedParameter(e3)),
                    Err::Failure(Error::NestedParameter(e4)),
                ] => {
                    assert_eq!(*e1, "{nest}");
                    assert_eq!(*e2, "{nest}");
                    assert_eq!(*e3, "{nest}");
                    assert_eq!(*e4, "{nest}");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_optional() {
            let err = [
                parameter(Spanned::new("{(nest)}")).expect_err("error"),
                parameter(Spanned::new("{before(nest)}")).expect_err("error"),
                parameter(Spanned::new("{(nest)after}")).expect_err("error"),
                parameter(Spanned::new("{bef(nest)aft}")).expect_err("error"),
            ];

            match err {
                [
                Err::Failure(Error::OptionalInParameter(e1)),
                Err::Failure(Error::OptionalInParameter(e2)),
                Err::Failure(Error::OptionalInParameter(e3)),
                Err::Failure(Error::OptionalInParameter(e4)),
                ] => {
                    assert_eq!(*e1, "(nest)");
                    assert_eq!(*e2, "(nest)");
                    assert_eq!(*e3, "(nest)");
                    assert_eq!(*e4, "(nest)");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_unescaped_reserved_char() {
            let err = [
                parameter(Spanned::new("{(opt}")).expect_err("error"),
                parameter(Spanned::new("{{nest}")).expect_err("error"),
                parameter(Spanned::new("{l/r}")).expect_err("error"),
            ];

            match err {
                [
                Err::Failure(Error::UnescapedReservedCharacter(e1)),
                Err::Failure(Error::UnescapedReservedCharacter(e2)),
                Err::Failure(Error::UnescapedReservedCharacter(e3)),
                ] => {
                    assert_eq!(*e1, "(");
                    assert_eq!(*e2, "{");
                    assert_eq!(*e3, "/");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_unfinished() {
            let err = [
                parameter(Spanned::new("{")).expect_err("error"),
                parameter(Spanned::new("{name ")).expect_err("error"),
            ];

            match err {
                [
                Err::Failure(Error::UnfinishedParameter(e1)),
                Err::Failure(Error::UnfinishedParameter(e2)),
                ] => {
                    assert_eq!(*e1, "{");
                    assert_eq!(*e2, "{");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }
    }
}
