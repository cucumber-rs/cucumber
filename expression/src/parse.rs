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
        Alternation, Alternative, Expression, Optional, Parameter, SingleExpr,
        Spanned,
    },
    combinator::{escaped0, map_err},
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
            Error::EscapedNonReservedCharacter(span.take(1)).failure()
        } else {
            e
        }
    })
}

fn is_text(c: char) -> bool {
    !RESERVED_CHARS.contains(c) || matches!(c, '}' | ')')
}

/// # Syntax
///
/// ```text
/// parameter := '{' name* '}'
/// name      := any character except '{' | '}' | '(' | '/'
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
/// {🦀}
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
/// - [`EscapedNonReservedCharacter`]
/// - [`NestedParameter`]
/// - [`OptionalInParameter`]
/// - [`UnescapedReservedCharacter`]
/// - [`UnfinishedParameter`]
///
/// [`Error`]: Err::Error
/// [`Failure`]: Err::Failure
/// [`EscapedNonReservedCharacter`]: Error::EscapedNonReservedCharacter
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
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
            }
            Some('(') => {
                if let Ok((_, opt)) = peek(optional)(input) {
                    return Error::OptionalInParameter(
                        input.take(opt.len() + 2),
                    )
                    .failure();
                }
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
            }
            Some(c) if RESERVED_CHARS.contains(c) => {
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
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

/// # Syntax
///
/// ```text
/// optional := '(' text+ ')'
/// text     := any character except '(' | ')' | '{' | '/'
/// ```
///
/// Note: `(`, `)`, `{`, `/` still can be used if escaped with `\`.
///
/// # Example
///
/// ```text
/// (name)
/// (with spaces)
/// (escaped \/\{\()
/// (no need to escape })
/// (🦀)
/// ```
///
/// # Errors
///
/// ## Recoverable [`Error`]s
///
/// - If `input` doesn't start with `(`
///
/// ## Irrecoverable [`Failure`]s
///
/// - [`AlternationInOptional`]
/// - [`EmptyOptional`]
/// - [`EscapedNonReservedCharacter`]
/// - [`NestedOptional`]
/// - [`ParameterInOptional`]
/// - [`UnescapedReservedCharacter`]
/// - [`UnfinishedOptional`]
///
/// [`Error`]: Err::Error
/// [`Failure`]: Err::Failure
/// [`AlternationInOptional`]: Error::AlternationInOptional
/// [`EmptyOptional`]: Error::EmptyOptional
/// [`EscapedNonReservedCharacter`]: Error::EscapedNonReservedCharacter
/// [`NestedOptional`]: Error::NestedOptional
/// [`ParameterInOptional`]: Error::ParameterInOptional
/// [`UnescapedReservedCharacter`]: Error::UnescapedReservedCharacter
/// [`UnfinishedOptional`]: Error::UnfinishedOptional
fn optional<'s>(
    input: Spanned<'s>,
) -> IResult<Spanned<'s>, Optional<'s>, Error<'s>> {
    let is_text = |c| !"(){\\/".contains(c);

    let fail = |input: Spanned<'s>, opening_brace| {
        match input.chars().next() {
            Some('(') => {
                if let Ok((_, (opt, ..))) = peek(tuple((
                    optional,
                    escaped_reserved_chars0(take_while(is_text)),
                    tag(")"),
                )))(input)
                {
                    return Error::NestedOptional(input.take(opt.0.len() + 2))
                        .failure();
                }
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
            }
            Some('{') => {
                if let Ok((_, par)) = peek(parameter)(input) {
                    return Error::ParameterInOptional(
                        input.take(par.len() + 2),
                    )
                    .failure();
                }
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
            }
            Some('/') => {
                return Error::AlternationInOptional(input.take(1)).failure();
            }
            Some(c) if RESERVED_CHARS.contains(c) => {
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
            }
            _ => {}
        }
        Error::UnfinishedOptional(opening_brace).failure()
    };

    let (input, opening_paren) = tag("(")(input)?;
    let (input, opt) = escaped_reserved_chars0(take_while(is_text))(input)?;
    let (input, _) = map_err(tag(")"), |_| fail(input, opening_paren))(input)?;

    if opt.is_empty() {
        return Err(Err::Failure(Error::EmptyOptional(opt)));
    }

    Ok((input, Optional(opt)))
}

/// # Syntax
///
/// ```text
/// alternative := optional | text*
/// text        := any character except ' ' | '(' | '{' | '/'
/// ```
///
/// Note: ` `, `(`, `{`, `/` still can be used if escaped with `\`.
///
/// # Example
///
/// ```text
/// text
/// escaped\ whitespace
/// no-need-to-escape)}
/// 🦀
/// (optional)
/// ```
///
/// Note: empty string is matched too.
///
/// # Errors
///
/// ## Irrecoverable [`Failure`]s
///
/// Any [`Failure`] of [`optional()`].
///
/// [`Failure`]: Err::Failure
fn alternative(input: Spanned) -> IResult<Spanned, Alternative, Error> {
    let is_text = |c| !" ({\\/".contains(c);

    alt((
        map(optional, Alternative::Optional),
        map(
            escaped_reserved_chars0(take_while(is_text)),
            Alternative::Text,
        ),
    ))(input)
}

/// # Example
///
/// ```text
/// left/right
/// left(opt)/(opt)right
/// escaped\ /text
/// no-need-to-escape)}/text
/// 🦀/⚙️
/// ```
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
    // Parameter
    NestedParameter(Spanned<'a>),
    OptionalInParameter(Spanned<'a>),
    UnfinishedParameter(Spanned<'a>),

    // Optional
    NestedOptional(Spanned<'a>),
    ParameterInOptional(Spanned<'a>),
    EmptyOptional(Spanned<'a>),
    AlternationInOptional(Spanned<'a>),
    UnfinishedOptional(Spanned<'a>),

    // Alternation
    EmptyAlternation(Spanned<'a>),
    OnlyOptionalInAlternation(Spanned<'a>),

    // General escaping
    UnescapedReservedCharacter(Spanned<'a>),
    EscapedNonReservedCharacter(Spanned<'a>),

    // nom
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
        alternative, optional, parameter, Alternative, Err, Error, ErrorKind,
        IResult, Spanned,
    };

    fn unwrap_parser<'s, T>(par: IResult<Spanned<'s>, T, Error<'s>>) -> T {
        let (rest, par) = par.expect("ok");
        assert_eq!(*rest, "");
        par
    }

    mod parameter {
        use super::{parameter, unwrap_parser, Err, Error, ErrorKind, Spanned};

        #[test]
        fn empty() {
            assert_eq!(**unwrap_parser(parameter(Spanned::new("{}"))), "");
        }

        #[test]
        fn named() {
            assert_eq!(
                **unwrap_parser(parameter(Spanned::new("{string}"))),
                "string",
            );
        }

        #[test]
        fn named_with_spaces() {
            assert_eq!(
                **unwrap_parser(parameter(Spanned::new("{with space}"))),
                "with space",
            );
        }

        #[test]
        fn named_with_escaped() {
            assert_eq!(
                **unwrap_parser(parameter(Spanned::new("{with \\{}"))),
                "with \\{",
            );
        }

        #[test]
        fn named_with_closing_paren() {
            assert_eq!(
                **unwrap_parser(parameter(Spanned::new("{with )}"))),
                "with )",
            );
        }

        #[allow(clippy::non_ascii_literal)]
        #[test]
        fn named_with_emoji() {
            assert_eq!(**unwrap_parser(parameter(Spanned::new("{🦀}"))), "🦀",);
        }

        #[test]
        fn errors_on_empty() {
            let span = Spanned::new("");
            assert_eq!(
                parameter(span),
                Err(Err::Error(Error::Other(span, ErrorKind::Tag))),
            );
        }

        #[test]
        fn fails_on_escaped_non_reserved() {
            let err = parameter(Spanned::new("{\\r}")).unwrap_err();

            match err {
                Err::Failure(Error::EscapedNonReservedCharacter(e)) => {
                    assert_eq!(*e, "\\");
                }
                _ => panic!("wrong error: {:?}", err),
            }
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
                #[rustfmt::skip]
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
                #[rustfmt::skip]
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
                parameter(Spanned::new("{(n(e)st)}")).expect_err("error"),
                parameter(Spanned::new("{{nest}")).expect_err("error"),
                parameter(Spanned::new("{l/r}")).expect_err("error"),
            ];

            match err {
                #[rustfmt::skip]
                [
                    Err::Failure(Error::UnescapedReservedCharacter(e1)),
                    Err::Failure(Error::UnescapedReservedCharacter(e2)),
                    Err::Failure(Error::UnescapedReservedCharacter(e3)),
                    Err::Failure(Error::UnescapedReservedCharacter(e4)),
                ] => {
                    assert_eq!(*e1, "(");
                    assert_eq!(*e2, "(");
                    assert_eq!(*e3, "{");
                    assert_eq!(*e4, "/");
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
                #[rustfmt::skip]
                [
                    Err::Failure(Error::UnfinishedParameter(e1)),
                    Err::Failure(Error::UnfinishedParameter(e2))
                ] => {
                    assert_eq!(*e1, "{");
                    assert_eq!(*e2, "{");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }
    }

    mod optional {
        use super::{optional, unwrap_parser, Err, Error, ErrorKind, Spanned};

        #[test]
        fn basic() {
            assert_eq!(
                **unwrap_parser(optional(Spanned::new("(string)"))),
                "string",
            );
        }

        #[test]
        fn with_spaces() {
            assert_eq!(
                **unwrap_parser(optional(Spanned::new("(with space)"))),
                "with space",
            );
        }

        #[test]
        fn with_escaped() {
            assert_eq!(
                **unwrap_parser(optional(Spanned::new("(with \\{)"))),
                "with \\{",
            );
        }

        #[test]
        fn with_closing_brace() {
            assert_eq!(
                **unwrap_parser(optional(Spanned::new("(with })"))),
                "with }",
            );
        }

        #[allow(clippy::non_ascii_literal)]
        #[test]
        fn with_emoji() {
            assert_eq!(**unwrap_parser(optional(Spanned::new("(🦀)"))), "🦀");
        }

        #[test]
        fn errors_on_empty() {
            let span = Spanned::new("");

            assert_eq!(
                optional(span),
                Err(Err::Error(Error::Other(span, ErrorKind::Tag))),
            );
        }

        #[test]
        fn fails_on_empty() {
            let err = optional(Spanned::new("()")).unwrap_err();

            match err {
                Err::Failure(Error::EmptyOptional(e)) => {
                    assert_eq!(*e, "\\");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_escaped_non_reserved() {
            let err = optional(Spanned::new("(\\r)")).unwrap_err();

            match err {
                Err::Failure(Error::EscapedNonReservedCharacter(e)) => {
                    assert_eq!(*e, "\\");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_nested() {
            let err = [
                optional(Spanned::new("((nest))")).expect_err("error"),
                optional(Spanned::new("(before(nest))")).expect_err("error"),
                optional(Spanned::new("((nest)after)")).expect_err("error"),
                optional(Spanned::new("(bef(nest)aft)")).expect_err("error"),
            ];

            match err {
                #[rustfmt::skip]
                [
                Err::Failure(Error::NestedOptional(e1)),
                Err::Failure(Error::NestedOptional(e2)),
                Err::Failure(Error::NestedOptional(e3)),
                Err::Failure(Error::NestedOptional(e4)),
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
        fn fails_on_parameter() {
            let err = [
                optional(Spanned::new("({nest})")).expect_err("error"),
                optional(Spanned::new("(before{nest})")).expect_err("error"),
                optional(Spanned::new("({nest}after)")).expect_err("error"),
                optional(Spanned::new("(bef{nest}aft)")).expect_err("error"),
            ];

            match err {
                #[rustfmt::skip]
                [
                Err::Failure(Error::ParameterInOptional(e1)),
                Err::Failure(Error::ParameterInOptional(e2)),
                Err::Failure(Error::ParameterInOptional(e3)),
                Err::Failure(Error::ParameterInOptional(e4)),
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
        fn fails_on_alternation() {
            let err = [
                optional(Spanned::new("(/)")).expect_err("error"),
                optional(Spanned::new("(bef/)")).expect_err("error"),
                optional(Spanned::new("(/aft)")).expect_err("error"),
                optional(Spanned::new("(bef/aft)")).expect_err("error"),
            ];

            match err {
                #[rustfmt::skip]
                [
                Err::Failure(Error::AlternationInOptional(e1)),
                Err::Failure(Error::AlternationInOptional(e2)),
                Err::Failure(Error::AlternationInOptional(e3)),
                Err::Failure(Error::AlternationInOptional(e4)),
                ] => {
                    assert_eq!(*e1, "/");
                    assert_eq!(*e2, "/");
                    assert_eq!(*e3, "/");
                    assert_eq!(*e4, "/");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_unescaped_reserved_char() {
            let err = [
                optional(Spanned::new("({opt)")).expect_err("error"),
                optional(Spanned::new("({n{e}st})")).expect_err("error"),
                optional(Spanned::new("((nest)")).expect_err("error"),
            ];

            match err {
                #[rustfmt::skip]
                [
                Err::Failure(Error::UnescapedReservedCharacter(e1)),
                Err::Failure(Error::UnescapedReservedCharacter(e2)),
                Err::Failure(Error::UnescapedReservedCharacter(e3)),
                ] => {
                    assert_eq!(*e1, "{");
                    assert_eq!(*e2, "{");
                    assert_eq!(*e3, "(");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_unfinished() {
            let err = [
                optional(Spanned::new("(")).expect_err("error"),
                optional(Spanned::new("(name ")).expect_err("error"),
            ];

            match err {
                #[rustfmt::skip]
                [
                Err::Failure(Error::UnfinishedOptional(e1)),
                Err::Failure(Error::UnfinishedOptional(e2))
                ] => {
                    assert_eq!(*e1, "(");
                    assert_eq!(*e2, "(");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }
    }

    mod alternative {
        use super::{
            alternative, unwrap_parser, Alternative, Err, Error, Spanned,
        };

        #[test]
        fn empty() {
            match unwrap_parser(alternative(Spanned::new(""))) {
                Alternative::Text(t) => assert_eq!(*t, ""),
                Alternative::Optional(_) => {
                    panic!("expected Alternative::Text")
                }
            }
        }

        #[allow(clippy::non_ascii_literal)]
        #[test]
        fn text() {
            match (
                unwrap_parser(alternative(Spanned::new("string"))),
                unwrap_parser(alternative(Spanned::new("🦀"))),
            ) {
                (Alternative::Text(t1), Alternative::Text(t2)) => {
                    assert_eq!(*t1, "string");
                    assert_eq!(*t2, "🦀");
                }
                _ => {
                    panic!("expected Alternative::Text")
                }
            }
        }

        #[test]
        fn escaped_spaces() {
            match (
                unwrap_parser(alternative(Spanned::new("bef\\ "))),
                unwrap_parser(alternative(Spanned::new("\\ aft"))),
                unwrap_parser(alternative(Spanned::new("bef\\ aft"))),
            ) {
                (
                    Alternative::Text(t1),
                    Alternative::Text(t2),
                    Alternative::Text(t3),
                ) => {
                    assert_eq!(*t1, "bef\\ ");
                    assert_eq!(*t2, "\\ aft");
                    assert_eq!(*t3, "bef\\ aft");
                }
                _ => {
                    panic!("expected Alternative::Text")
                }
            }
        }

        #[test]
        fn optional() {
            match unwrap_parser(alternative(Spanned::new("(opt)"))) {
                Alternative::Optional(t) => {
                    assert_eq!(**t, "opt");
                }
                _ => {
                    panic!("expected Alternative::Optional")
                }
            }
        }

        #[test]
        fn not_captures_unescaped_whitespace() {
            match alternative(Spanned::new("text ")) {
                Ok((rest, matched)) => {
                    assert_eq!(*rest, " ");

                    match matched {
                        Alternative::Text(t) => assert_eq!(*t, "text"),
                        Alternative::Optional(_) => {
                            panic!("expected Alternative::Text")
                        }
                    }
                }
                Err(..) => panic!("expected ok"),
            }
        }

        #[test]
        fn fails_on_unfinished_optional() {
            let err = (
                alternative(Spanned::new("(")).unwrap_err(),
                alternative(Spanned::new("(opt")).unwrap_err(),
            );

            match err {
                (
                    Err::Failure(Error::UnfinishedOptional(e1)),
                    Err::Failure(Error::UnfinishedOptional(e2)),
                ) => {
                    assert_eq!(*e1, "(");
                    assert_eq!(*e2, "(");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_escaped_non_reserved() {
            let err = (
                alternative(Spanned::new("(\\r)")).unwrap_err(),
                alternative(Spanned::new("\\r")).unwrap_err(),
            );

            match err {
                (
                    Err::Failure(Error::EscapedNonReservedCharacter(e1)),
                    Err::Failure(Error::EscapedNonReservedCharacter(e2)),
                ) => {
                    assert_eq!(*e1, "\\");
                    assert_eq!(*e2, "\\");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }
    }
}
