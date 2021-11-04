use std::ops::RangeFrom;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::one_of,
    combinator::{map, peek, verify},
    error::{ErrorKind, ParseError},
    multi::{many0, many1, separated_list1},
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
pub const RESERVED_CHARS: &str = r#"{}()\/ "#;

/// Matches `normal` and [`RESERVED_CHARS`] escaped with `\`.
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
/// - If `normal` parser fails
/// - [`EscapedNonReservedCharacter`]
///
/// [`Error`]: Err::Error
/// [`EscapedNonReservedCharacter`]: Error::EscapedNonReservedCharacter
/// [`Failure`]: Err::Failure
pub fn escaped_reserved_chars0<'a, Input: 'a, F, O1>(
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
pub fn parameter<'s>(
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
/// optional         := '(' text_in_optional+ ')'
/// text_in_optional := any character except '(' | ')' | '{' | '/'
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
pub fn optional<'s>(
    input: Spanned<'s>,
) -> IResult<Spanned<'s>, Optional<'s>, Error<'s>> {
    let is_text = |c| !"(){\\/".contains(c);

    let original_input = input;
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
        return Err(Err::Failure(Error::EmptyOptional(original_input.take(2))));
    }

    Ok((input, Optional(opt)))
}

/// # Syntax
///
/// ```text
/// alternative             := optional | text+
/// text_without_whitespace := any character except ' ' | '(' | '{' | '/'
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
/// # Errors
///
/// ## Irrecoverable [`Failure`]s
///
/// Any [`Failure`] of [`optional()`].
///
/// [`Failure`]: Err::Failure
pub fn alternative(input: Spanned) -> IResult<Spanned, Alternative, Error> {
    let is_text = |c| !" ({\\/".contains(c);

    alt((
        map(optional, Alternative::Optional),
        map(
            verify(escaped_reserved_chars0(take_while(is_text)), |p| {
                !p.is_empty()
            }),
            Alternative::Text,
        ),
    ))(input)
}

/// # Syntax
///
/// ```text
/// alternation             := single_alternation (`/` single_alternation)+
/// single_alternation      := ((text+ optional*) | (optional+ text+))+
/// ```
///
/// # Example
///
/// ```text
/// left/right
/// left(opt)/(opt)right
/// escaped\ /text
/// no-need-to-escape)}/text
/// 🦀/⚙️
/// ```
///
/// # Errors
///
/// ## Recoverable [`Error`]s
///
/// - If `input` doesn't have `/`
///
/// ## Irrecoverable [`Failure`]s
///
/// - Any [`Failure`] of [`optional()`]
/// - [`EmptyAlternation`]
/// - [`OnlyOptionalInAlternation`]
///
/// [`Error`]: Err::Error
/// [`Failure`]: Err::Failure
/// [`EmptyAlternation`]: Error::EmptyAlternation
/// [`OnlyOptionalInAlternation`]: Error::OnlyOptionalInAlternation
pub fn alternation(input: Spanned) -> IResult<Spanned, Alternation, Error> {
    let (rest, alt) = match separated_list1(tag("/"), many1(alternative))(input)
    {
        Ok((rest, alt)) => {
            if let Ok((_, slash)) = peek::<_, _, Error, _>(tag("/"))(rest) {
                Err(Error::EmptyAlternation(slash).failure())
            } else if alt.len() == 1 {
                Err(Err::Error(Error::Other(rest, ErrorKind::Tag)))
            } else {
                Ok((rest, Alternation(alt)))
            }
        }
        Err(Err::Error(Error::Other(sp, ErrorKind::Many1)))
            if peek::<_, _, Error, _>(tag("/"))(sp).is_ok() =>
        {
            Err(Error::EmptyAlternation(sp.take(1)).failure())
        }
        Err(e) => Err(e),
    }?;

    alt.contains_only_optional()
        .then(|| {
            Err(Error::OnlyOptionalInAlternation(input.take(alt.span_len()))
                .failure())
        })
        .unwrap_or(Ok((rest, alt)))
}

/// # Syntax
///
/// ```text
/// single_expression := alternation
///                      | optional
///                      | parameter
///                      | text_without_whitespace*
///                      | whitespace
/// ```
///
/// # Example
///
/// ```text
/// text(opt)/text
/// (opt)
/// {string}
/// text
/// ```
///
/// Note: empty string is matched too.
///
/// # Errors
///
/// ## Irrecoverable [`Failure`]s
///
/// Any [`Failure`] of [`alternation()`], [`optional()`] or [`parameter()`].
///
/// [`Error`]: Err::Error
/// [`Failure`]: Err::Failure
/// [`EmptyAlternation`]: Error::EmptyAlternation
/// [`OnlyOptionalInAlternation`]: Error::OnlyOptionalInAlternation
pub fn single_expression(
    input: Spanned,
) -> IResult<Spanned, SingleExpr, Error> {
    let is_text = |c| !" ({\\/".contains(c);

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

/// # Syntax
///
/// ```text
/// expression := single_expression*
/// ```
///
/// # Example
///
/// ```text
/// text(opt)/text
/// (opt)
/// {string}
/// text
/// ```
///
/// Note: empty string is matched too.
///
/// # Errors
///
/// ## Irrecoverable [`Failure`]s
///
/// Any [`Failure`] of [`alternation()`], [`optional()`] or [`parameter()`].
///
/// [`Error`]: Err::Error
/// [`Failure`]: Err::Failure
/// [`EmptyAlternation`]: Error::EmptyAlternation
/// [`OnlyOptionalInAlternation`]: Error::OnlyOptionalInAlternation
pub fn expression(input: Spanned) -> IResult<Spanned, Expression, Error> {
    map(many0(single_expression), Expression)(input)
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
        alternation, alternative, expression, optional, parameter, Alternative,
        Err, Error, ErrorKind, IResult, Spanned,
    };

    fn eq(left: impl AsRef<str>, right: impl AsRef<str>) {
        assert_eq!(
            left.as_ref()
                .replace(' ', "")
                .replace('\n', "")
                .replace('\t', ""),
            right
                .as_ref()
                .replace(' ', "")
                .replace('\n', "")
                .replace('\t', ""),
        );
    }

    fn unwrap_parser<'s, T>(par: IResult<Spanned<'s>, T, Error<'s>>) -> T {
        let (rest, par) = par.unwrap();
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
                    assert_eq!(*e, "()");
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
            alternative, unwrap_parser, Alternative, Err, Error, ErrorKind,
            Spanned,
        };

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
        fn errors_on_empty() {
            match alternative(Spanned::new("")).unwrap_err() {
                Err::Error(Error::Other(_, ErrorKind::Alt)) => {}
                e => panic!("wrong error"),
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

    mod alternation {
        use super::{
            alternation, eq, unwrap_parser, Err, Error, ErrorKind, Spanned,
        };

        #[allow(clippy::non_ascii_literal)]
        #[test]
        fn basic() {
            let ast = format!(
                "{:?}",
                unwrap_parser(alternation(Spanned::new("l/🦀")))
            );

            eq(
                ast,
                r#"Alternation (
                    [
                        [
                            Text (
                                LocatedSpan {
                                    offset: 0,
                                    line: 1,
                                    fragment: "l",
                                    extra: ()
                                }
                            )
                        ],
                        [
                            Text (
                                LocatedSpan {
                                    offset: 2,
                                    line: 1,
                                    fragment: "🦀",
                                    extra: ()
                                }
                            )
                        ]
                    ]
                )"#,
            );
        }

        #[test]
        fn with_optionals() {
            let ast = format!(
                "{:?}",
                unwrap_parser(alternation(Spanned::new(
                    "l(opt)/(opt)r/l(opt)r"
                ))),
            );

            eq(
                ast,
                r#"Alternation (
                    [
                        [
                            Text (
                                LocatedSpan {
                                    offset: 0,
                                    line: 1,
                                    fragment: "l",
                                    extra: ()
                                }
                            ),
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 2,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            )
                        ],
                        [
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 8,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            ),
                            Text (
                                LocatedSpan {
                                    offset: 12,
                                    line: 1,
                                    fragment: "r",
                                    extra: ()
                                }
                            )
                        ],
                        [
                            Text (
                                LocatedSpan {
                                    offset: 14,
                                    line: 1,
                                    fragment: "l",
                                    extra: ()
                                }
                            ),
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 16,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            ),
                            Text (
                                LocatedSpan {
                                    offset: 20,
                                    line: 1,
                                    fragment: "r",
                                    extra: ()
                                }
                            )
                        ]
                    ]
                )"#,
            );
        }

        #[test]
        fn with_more_optionals() {
            let ast = format!(
                "{:?}",
                unwrap_parser(alternation(Spanned::new(
                    "l(opt)(opt)/(opt)(opt)r/(opt)m(opt)"
                ))),
            );

            eq(
                ast,
                r#"Alternation (
                    [
                        [
                            Text (
                                LocatedSpan {
                                    offset: 0,
                                    line: 1,
                                    fragment: "l",
                                    extra: ()
                                }
                            ),
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 2,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            ),
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 7,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            )
                        ],
                        [
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 13,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            ),
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 18,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            ),
                            Text (
                                LocatedSpan {
                                    offset: 22,
                                    line: 1,
                                    fragment: "r",
                                    extra: ()
                                }
                            )
                        ],
                        [
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 25,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            ),
                            Text (
                                LocatedSpan {
                                    offset: 29,
                                    line: 1,
                                    fragment: "m",
                                    extra: ()
                                }
                            ),
                            Optional (
                                Optional (
                                    LocatedSpan {
                                        offset: 31,
                                        line: 1,
                                        fragment: "opt",
                                        extra: ()
                                    }
                                )
                            )
                        ]
                    ]
                )"#,
            );
        }

        #[test]
        fn errors_without_slash() {
            match (
                alternation(Spanned::new("")).unwrap_err(),
                alternation(Spanned::new("{par}")).unwrap_err(),
                alternation(Spanned::new("text")).unwrap_err(),
                alternation(Spanned::new("(opt)")).unwrap_err(),
            ) {
                (
                    Err::Error(Error::Other(_, ErrorKind::Many1)),
                    Err::Error(Error::Other(_, ErrorKind::Many1)),
                    Err::Error(Error::Other(_, ErrorKind::Tag)),
                    Err::Error(Error::Other(_, ErrorKind::Tag)),
                ) => {}
                _ => panic!("wrong err"),
            }
        }

        #[test]
        fn fails_on_empty_alternation() {
            let err = (
                alternation(Spanned::new("/")).unwrap_err(),
                alternation(Spanned::new("l/")).unwrap_err(),
                alternation(Spanned::new("/r")).unwrap_err(),
                alternation(Spanned::new("l/m/")).unwrap_err(),
                alternation(Spanned::new("l//r")).unwrap_err(),
                alternation(Spanned::new("/m/r")).unwrap_err(),
            );

            match err {
                (
                    Err::Failure(Error::EmptyAlternation(e1)),
                    Err::Failure(Error::EmptyAlternation(e2)),
                    Err::Failure(Error::EmptyAlternation(e3)),
                    Err::Failure(Error::EmptyAlternation(e4)),
                    Err::Failure(Error::EmptyAlternation(e5)),
                    Err::Failure(Error::EmptyAlternation(e6)),
                ) => {
                    assert_eq!(*e1, "/");
                    assert_eq!(*e2, "/");
                    assert_eq!(*e3, "/");
                    assert_eq!(*e4, "/");
                    assert_eq!(*e5, "/");
                    assert_eq!(*e6, "/");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }

        #[test]
        fn fails_on_only_optional() {
            let err = (
                alternation(Spanned::new("text/(opt)")).unwrap_err(),
                alternation(Spanned::new("text/(opt)(opt)")).unwrap_err(),
                alternation(Spanned::new("(opt)/text")).unwrap_err(),
                alternation(Spanned::new("(opt)/(opt)")).unwrap_err(),
            );

            match err {
                (
                    Err::Failure(Error::OnlyOptionalInAlternation(e1)),
                    Err::Failure(Error::OnlyOptionalInAlternation(e2)),
                    Err::Failure(Error::OnlyOptionalInAlternation(e3)),
                    Err::Failure(Error::OnlyOptionalInAlternation(e4)),
                ) => {
                    assert_eq!(*e1, "text/(opt)");
                    assert_eq!(*e2, "text/(opt)(opt)");
                    assert_eq!(*e3, "(opt)/text");
                    assert_eq!(*e4, "(opt)/(opt)");
                }
                _ => panic!("wrong error: {:?}", err),
            }
        }
    }

    // all test examples from: https://bit.ly/3q6m53v
    mod expression {
        use super::{eq, expression, unwrap_parser, Err, Error, Spanned};

        #[test]
        fn allows_escaped_optional_parameter_types() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new("\\({int})")))
            );
            eq(
                ast,
                r#"Expression (
                    [
                        Text (
                            LocatedSpan {
                                offset: 0,
                                line: 1,
                                fragment: "\\(",
                                extra: ()
                            }
                        ),
                        Parameter (
                            Parameter (
                                LocatedSpan {
                                    offset: 3,
                                    line: 1,
                                    fragment: "int",
                                    extra: ()
                                }
                            )
                        ),
                        Text (
                            LocatedSpan {
                                offset: 7,
                                line: 1,
                                fragment: ")",
                                extra: ()
                            }
                        )
                    ]
                )"#,
            );
        }

        #[test]
        fn allows_parameter_type_in_alternation() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new("a/i{int}n/y")))
            );
            eq(
                ast,
                r#"Expression(
                    [
                        Alternation (
                            Alternation (
                                [
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 0,
                                                line: 1,
                                                fragment: "a",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 2,
                                                line: 1,
                                                fragment: "i",
                                                extra: ()
                                            }
                                        )
                                    ]
                                ]
                            )
                        ),
                        Parameter (
                            Parameter (
                                LocatedSpan {
                                    offset: 4,
                                    line: 1,
                                    fragment: "int",
                                    extra: ()
                                }
                            )
                        ),
                        Alternation (
                            Alternation (
                                [
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 8,
                                                line: 1,
                                                fragment: "n",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 10,
                                                line: 1,
                                                fragment: "y",
                                                extra: ()
                                            }
                                        )
                                    ]
                                ]
                            )
                        )
                    ]
                )"#,
            );
        }

        #[test]
        fn does_allow_parameter_adjacent_to_alternation() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new("{int}st/nd/rd/th")))
            );
            eq(
                ast,
                r#"Expression (
                    [
                        Parameter (
                            Parameter (
                                LocatedSpan {
                                    offset: 1,
                                    line: 1,
                                    fragment: "int",
                                    extra: ()
                                }
                            )
                        ),
                        Alternation (
                            Alternation (
                                [
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 5,
                                                line: 1,
                                                fragment:
                                                "st",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 8,
                                                line: 1,
                                                fragment: "nd",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 11,
                                                line: 1,
                                                fragment: "rd",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 14,
                                                line: 1,
                                                fragment: "th",
                                                extra: ()
                                            }
                                        )
                                    ]
                                ]
                            )
                        )
                    ]
                )"#,
            );
        }

        #[test]
        fn does_not_allow_alternation_in_optional() {
            match expression(Spanned::new("three( brown/black) mice"))
                .unwrap_err()
            {
                Err::Failure(Error::AlternationInOptional(s)) => {
                    assert_eq!(*s, "/")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[rustfmt::skip]
        #[test]
        fn does_not_allow_alternation_with_empty_alternative_by_adjacent_left_parameter() {
            match expression(Spanned::new("{int}/x")).unwrap_err() {
                Err::Failure(Error::EmptyAlternation(s)) => {
                    assert_eq!(*s, "/")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[rustfmt::skip]
        #[test]
        fn does_not_allow_alternation_with_empty_alternative_by_adjacent_optional() {
            match expression(Spanned::new("three (brown)/black mice")).unwrap_err() {
                Err::Failure(Error::OnlyOptionalInAlternation(s)) => {
                    assert_eq!(*s, "(brown)/black")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[rustfmt::skip]
        #[test]
        fn does_not_allow_alternation_with_empty_alternative_by_adjacent_right_parameter() {
            match expression(Spanned::new("x/{int}")).unwrap_err() {
                Err::Failure(Error::EmptyAlternation(s)) => {
                    assert_eq!(*s, "/")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_alternation_with_empty_alternative() {
            match expression(Spanned::new("three brown//black mice"))
                .unwrap_err()
            {
                Err::Failure(Error::EmptyAlternation(s)) => {
                    assert_eq!(*s, "/")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_empty_optional() {
            match expression(Spanned::new("three () mice")).unwrap_err() {
                Err::Failure(Error::EmptyOptional(s)) => {
                    assert_eq!(*s, "()")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_nested_optional() {
            match expression(Spanned::new("(a(b))")).unwrap_err() {
                Err::Failure(Error::NestedOptional(s)) => {
                    assert_eq!(*s, "(b)")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_optional_parameter_types() {
            match expression(Spanned::new("({int})")).unwrap_err() {
                Err::Failure(Error::ParameterInOptional(s)) => {
                    assert_eq!(*s, "{int}")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_parameter_name_with_reserved_characters() {
            match expression(Spanned::new("{(string)}")).unwrap_err() {
                Err::Failure(Error::OptionalInParameter(s)) => {
                    assert_eq!(*s, "(string)")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_unfinished_parenthesis_1() {
            match expression(Spanned::new(
                "three (exceptionally\\) {string\\} mice",
            ))
            .unwrap_err()
            {
                Err::Failure(Error::UnescapedReservedCharacter(s)) => {
                    assert_eq!(*s, "{")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_unfinished_parenthesis_2() {
            match expression(Spanned::new(
                "three (exceptionally\\) {string} mice",
            ))
            .unwrap_err()
            {
                Err::Failure(Error::ParameterInOptional(s)) => {
                    assert_eq!(*s, "{string}")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_unfinished_parenthesis_3() {
            match expression(Spanned::new(
                "three ((exceptionally\\) strong) mice",
            ))
            .unwrap_err()
            {
                Err::Failure(Error::UnescapedReservedCharacter(s)) => {
                    assert_eq!(*s, "(")
                }
                e => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn matches_alternation() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new(
                    "mice/rats and rats\\/mice"
                )))
            );
            eq(
                ast,
                r#"Expression (
                    [
                        Alternation (
                            Alternation (
                                [
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 0,
                                                line: 1,
                                                fragment: "mice",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 5,
                                                line: 1,
                                                fragment: "rats",
                                                extra: ()
                                            }
                                        )
                                    ]
                                ]
                            )
                        ),
                        Space,
                        Text (
                            LocatedSpan {
                                offset: 10,
                                line: 1,
                                fragment: "and",
                                extra: ()
                            }
                        ),
                        Space,
                        Text (
                            LocatedSpan {
                                offset: 14,
                                line: 1,
                                fragment: "rats\\/mice",
                                extra: ()
                            }
                        )
                    ]
                )"#,
            );
        }

        #[test]
        fn empty() {
            let ast =
                format!("{:?}", unwrap_parser(expression(Spanned::new(""))));
            eq(ast, r#"Expression([])"#);
        }
    }
}
