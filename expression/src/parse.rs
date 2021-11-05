// Copyright (c) 2021  Brendan Molloy <brendan@bbqsrc.net>,
//                     Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                     Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Cucumber expression][1] [AST][2] parsers definitions.
//!
//! [1]: https://github.com/cucumber/cucumber-expressions#readme
//! [2]: https://en.wikipedia.org/wiki/Abstract_syntax_tree

use std::{fmt::Display, ops::RangeFrom};

use derive_more::{Display, Error};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::one_of,
    combinator::{map, peek, verify},
    error::{ErrorKind, ParseError},
    multi::{many0, many1, separated_list1},
    sequence::tuple,
    AsChar, Compare, Err, FindToken, IResult, InputIter, InputLength,
    InputTake, InputTakeAtPosition, Needed, Offset, Parser, Slice,
};

use crate::{
    ast::{
        Alternation, Alternative, Expression, Optional, Parameter,
        SingleExpression,
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
fn escaped_reserved_chars0<'a, Input: 'a, F, O1>(
    normal: F,
) -> impl FnMut(Input) -> IResult<Input, Input, Error<Input>>
where
    Input: Clone
        + Display
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition
        + Slice<RangeFrom<usize>>
        + InputIter,
    <Input as InputIter>::Item: AsChar + Copy,
    F: Parser<Input, O1, Error<Input>>,
    Error<Input>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    map_err(escaped0(normal, '\\', one_of(RESERVED_CHARS)), |e| {
        if let Err::Error(Error::Other(span, ErrorKind::Escaped)) = e {
            let span =
                (span.input_len() > 0).then(|| span.take(1)).unwrap_or(span);
            Error::EscapedNonReservedCharacter(span).failure()
        } else {
            e
        }
    })
}

/// # Syntax
///
/// ```text
/// parameter       := '{' (name | '\' name_to_escape)* '}'
/// name            := ^name_to_escape
/// name_to_escape  := '{' | '}' | '(' | '/' | '\'
/// ```
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
pub fn parameter<'a, Input: 'a>(
    input: Input,
) -> IResult<Input, Parameter<Input>, Error<Input>>
where
    Input: Clone
        + Display
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition<Item = char>
        + Slice<RangeFrom<usize>>
        + InputIter
        + for<'s> Compare<&'s str>,
    <Input as InputIter>::Item: AsChar + Copy,
    Error<Input>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    let is_name = |c| !"{}(\\/".contains(c);

    let fail = |input: Input, opening_brace| {
        match input.iter_elements().next().map(AsChar::as_char) {
            Some('{') => {
                if let Ok((_, (par, ..))) = peek(tuple((
                    parameter,
                    escaped_reserved_chars0(take_while(is_name)),
                    tag("}"),
                )))(input.clone())
                {
                    return Error::NestedParameter(
                        input.take(par.0.input_len() + 2),
                    )
                    .failure();
                }
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
            }
            Some('(') => {
                if let Ok((_, opt)) = peek(optional)(input.clone()) {
                    return Error::OptionalInParameter(
                        input.take(opt.0.input_len() + 2),
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
    let (input, _) =
        map_err(tag("}"), |_| fail(input.clone(), opening_brace.clone()))(
            input.clone(),
        )?;

    Ok((input, Parameter(par_name)))
}

/// # Syntax
///
/// ```text
/// optional           := '(' (text_in_optional | '\' optional_to_escape)+ ')'
/// text_in_optional   := ^optional_to_escape
/// optional_to_escape := '(' | ')' | '{' | '/' | '\'
/// ```
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
pub fn optional<'a, Input: 'a>(
    input: Input,
) -> IResult<Input, Optional<Input>, Error<Input>>
where
    Input: Clone
        + Display
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition<Item = char>
        + Slice<RangeFrom<usize>>
        + InputIter
        + for<'s> Compare<&'s str>,
    <Input as InputIter>::Item: AsChar + Copy,
    Error<Input>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    let is_text_in_optional = |c| !"(){\\/".contains(c);

    let fail = |input: Input, opening_brace| {
        match input.iter_elements().next().map(AsChar::as_char) {
            Some('(') => {
                if let Ok((_, (opt, ..))) = peek(tuple((
                    optional,
                    escaped_reserved_chars0(take_while(is_text_in_optional)),
                    tag(")"),
                )))(input.clone())
                {
                    return Error::NestedOptional(
                        input.take(opt.0.input_len() + 2),
                    )
                    .failure();
                }
                return Error::UnescapedReservedCharacter(input.take(1))
                    .failure();
            }
            Some('{') => {
                if let Ok((_, par)) = peek(parameter)(input.clone()) {
                    return Error::ParameterInOptional(
                        input.take(par.0.input_len() + 2),
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

    let original_input = input.clone();
    let (input, opening_paren) = tag("(")(input)?;
    let (input, opt) =
        escaped_reserved_chars0(take_while(is_text_in_optional))(input)?;
    let (input, _) =
        map_err(tag(")"), |_| fail(input.clone(), opening_paren.clone()))(
            input.clone(),
        )?;

    if opt.input_len() == 0 {
        return Err(Err::Failure(Error::EmptyOptional(original_input.take(2))));
    }

    Ok((input, Optional(opt)))
}

/// # Syntax
///
/// ```text
/// alternative             := optional
///                            | (text_without_whitespace
///                               | '\' whitespace_and_special)+
/// text_without_whitespace := ^whitespace_and_special
/// whitespace_and_special  := ' ' | '(' | '{' | '/' | '\'
/// ```
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
pub fn alternative<'a, Input: 'a>(
    input: Input,
) -> IResult<Input, Alternative<Input>, Error<Input>>
where
    Input: Clone
        + Display
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition<Item = char>
        + Slice<RangeFrom<usize>>
        + InputIter
        + for<'s> Compare<&'s str>,
    <Input as InputIter>::Item: AsChar + Copy,
    Error<Input>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    let is_text_without_whitespace = |c| !" ({\\/".contains(c);

    alt((
        map(optional, Alternative::Optional),
        map(
            verify(
                escaped_reserved_chars0(take_while(is_text_without_whitespace)),
                |p| p.input_len() > 0,
            ),
            Alternative::Text,
        ),
    ))(input)
}

/// # Syntax
///
/// ```text
/// alternation        := single_alternation (`/` single_alternation)+
/// single_alternation := ((text_without_whitespace+ optional*)
///                         | (optional+ text_without_whitespace+))+
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
pub fn alternation<Input>(
    input: Input,
) -> IResult<Input, Alternation<Input>, Error<Input>>
where
    Input: Clone
        + Display
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition<Item = char>
        + Slice<RangeFrom<usize>>
        + InputIter
        + for<'s> Compare<&'s str>,
    <Input as InputIter>::Item: AsChar + Copy,
    Error<Input>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    let original_input = input.clone();
    let (rest, alt) = match separated_list1(tag("/"), many1(alternative))(input)
    {
        Ok((rest, alt)) => {
            if let Ok((_, slash)) =
                peek::<_, _, Error<Input>, _>(tag("/"))(rest.clone())
            {
                Err(Error::EmptyAlternation(slash).failure())
            } else if alt.len() == 1 {
                Err(Err::Error(Error::Other(rest, ErrorKind::Tag)))
            } else {
                Ok((rest, Alternation(alt)))
            }
        }
        Err(Err::Error(Error::Other(sp, ErrorKind::Many1)))
            if peek::<_, _, Error<Input>, _>(tag("/"))(sp.clone()).is_ok() =>
        {
            Err(Error::EmptyAlternation(sp.take(1)).failure())
        }
        Err(e) => Err(e),
    }?;

    alt.contains_only_optional()
        .then(|| {
            Err(Error::OnlyOptionalInAlternation(
                original_input.take(alt.span_len()),
            )
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
///                      | text_without_whitespace+
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
pub fn single_expression<'a, Input: 'a>(
    input: Input,
) -> IResult<Input, SingleExpression<Input>, Error<Input>>
where
    Input: Clone
        + Display
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition<Item = char>
        + Slice<RangeFrom<usize>>
        + InputIter
        + for<'s> Compare<&'s str>,
    <Input as InputIter>::Item: AsChar + Copy,
    Error<Input>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    let is_text_without_whitespace = |c| !" ({\\/".contains(c);

    alt((
        map(alternation, SingleExpression::Alternation),
        map(optional, SingleExpression::Optional),
        map(parameter, SingleExpression::Parameter),
        map(
            verify(
                escaped_reserved_chars0(take_while(is_text_without_whitespace)),
                |s| s.input_len() > 0,
            ),
            SingleExpression::Text,
        ),
        map(tag(" "), |_| SingleExpression::Whitespace),
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
pub fn expression<'a, Input: 'a>(
    input: Input,
) -> IResult<Input, Expression<Input>, Error<Input>>
where
    Input: Clone
        + Display
        + Offset
        + InputLength
        + InputTake
        + InputTakeAtPosition<Item = char>
        + Slice<RangeFrom<usize>>
        + InputIter
        + for<'s> Compare<&'s str>,
    <Input as InputIter>::Item: AsChar + Copy,
    Error<Input>: ParseError<Input>,
    for<'s> &'s str: FindToken<<Input as InputIter>::Item>,
{
    map(many0(single_expression), Expression)(input)
}

/// Possible parsing errors.
#[derive(Debug, Display, Error, Eq, PartialEq)]
pub enum Error<Input>
where
    Input: Display,
{
    /// Nested [`Parameter`]s.
    #[display(
        fmt = "\
        {}\n\
        A parameter may not contain an other parameter.\n\
        If you did not mean to use an optional type you can use '\\{{' to \
        escape the '{{'. For more complicated expressions consider using a \
        regular expression instead.",
        _0
    )]
    NestedParameter(#[error(not(source))] Input),

    /// [`Optional`] inside [`Parameter`].
    #[display(
        fmt = "\
        {}\n\
        A parameter may not contain an optional type.\n\
        If you did not mean to use an parameter type you can use '\\(' to \
        escape the '('.",
        _0
    )]
    OptionalInParameter(#[error(not(source))] Input),

    /// Unfinished [`Parameter`].
    #[display(
        fmt = "\
        {}\n\
        The '{{' does not have a matching '}}'.\n\
        If you did not intend to use a parameter you can use '\\{{' to escape \
        the '{{'.",
        _0
    )]
    UnfinishedParameter(#[error(not(source))] Input),

    /// Nested [`Optional`].
    #[display(
        fmt = "\
        {}\n\
        An optional may not contain an other optional.\n\
        If you did not mean to use an optional type you can use '\\(' to \
        escape the '('. For more complicated expressions consider using a \
        regular expression instead.",
        _0
    )]
    NestedOptional(#[error(not(source))] Input),

    /// [`Parameter`] inside [`Optional`].
    #[display(
        fmt = "\
        {}\n\
        An optional may not contain a parameter type.\n\
        If you did not mean to use an parameter type you can use '\\{{' to \
        escape the '{{'.",
        _0
    )]
    ParameterInOptional(#[error(not(source))] Input),

    /// Empty [`Optional`].
    #[display(
        fmt = "\
        {}\n\
        An optional must contain some text.\n\
        If you did not mean to use an optional you can use '\\(' to escape the \
        '('.",
        _0
    )]
    EmptyOptional(#[error(not(source))] Input),

    /// [`Alternation`] inside [`Optional`].
    #[display(
        fmt = "\
        {}\n\
        An alternation can not be used inside an optional.\n\
        You can use '\\/' to escape the '/'.",
        _0
    )]
    AlternationInOptional(#[error(not(source))] Input),

    /// Unfinished [`Optional`].
    #[display(
        fmt = "\
        {}\n\
        The '(' does not have a matching ')'.\n\
        If you did not intend to use an optional you can use '\\(' to escape \
        the '('.",
        _0
    )]
    UnfinishedOptional(#[error(not(source))] Input),

    /// Empty [`Alternation`].
    #[display(
        fmt = "\
        {}\n\
        Alternative may not be empty.\n\
        If you did not mean to use an alternative you can use '\\/' to escape \
        the '/'.",
        _0
    )]
    EmptyAlternation(#[error(not(source))] Input),

    /// Only [`Optional`] inside [`Alternation`].
    #[display(
        fmt = "\
        {}\n\
        An alternative may not exclusively contain optionals.\n\
        If you did not mean to use an optional you can use '\\(' to escape the \
        '('.",
        _0
    )]
    OnlyOptionalInAlternation(#[error(not(source))] Input),

    /// Unescaped [`RESERVED_CHARS`].
    #[display(
        fmt = "\
        {}\n\
        Unescaped reserved character.\n\
        You can use an '\\' to escape it.",
        _0
    )]
    UnescapedReservedCharacter(#[error(not(source))] Input),

    /// Escaped non-[`RESERVED_CHARS`].
    #[display(
        fmt = "\
        {}\n\
        Only the characters '{{', '}}', '(', ')', '\\', '/' and whitespace can \
        be escaped.\n\
        If you did mean to use an '\\' you can use '\\\\' to escape it.",
        _0
    )]
    EscapedNonReservedCharacter(#[error(not(source))] Input),

    /// Unknown error.
    #[display(
        fmt = "\
        {}\n\
        Unknown parsing error.",
        _0
    )]
    Other(#[error(not(source))] Input, ErrorKind),

    /// Parsing requires more data.
    #[display(
        fmt = "{}",
        "match _0 {\
            Needed::Size(n) => format!(\"Parsing requires {} bytes/chars\", n),\
            Needed::Unknown => \"Parsing requires more data\".to_owned(),\
    }"
    )]
    Needed(#[error(not(source))] Needed),
}

impl<Input: Display> Error<Input> {
    /// Converts this [`enum@Error`] into [`Failure`].
    ///
    /// [`Failure`]: Err::Failure
    fn failure(self) -> Err<Self> {
        Err::Failure(self)
    }
}

impl<Input: Display> ParseError<Input> for Error<Input> {
    fn from_error_kind(input: Input, kind: ErrorKind) -> Self {
        Self::Other(input, kind)
    }

    fn append(input: Input, kind: ErrorKind, other: Self) -> Self {
        if let Self::Other(..) = other {
            Self::from_error_kind(input, kind)
        } else {
            other
        }
    }
}

#[cfg(test)]
mod spec {
    use nom::{error::ErrorKind, Err, IResult};

    use crate::{
        parse::{alternation, alternative, expression, optional, parameter},
        Alternative, Error, Spanned,
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

    fn unwrap_parser<'s, T>(
        par: IResult<Spanned<'s>, T, Error<Spanned<'s>>>,
    ) -> T {
        let (rest, par) =
            par.unwrap_or_else(|e| panic!("Expected Ok, found Err: {}", e));
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
                Err::Incomplete(_) | Err::Error(_) | Err::Failure(_) => {
                    panic!("wrong error: {:?}", err)
                }
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
                Err::Incomplete(_) | Err::Error(_) | Err::Failure(_) => {
                    panic!("wrong error: {:?}", err)
                }
            }
        }

        #[test]
        fn fails_on_escaped_non_reserved() {
            let err = optional(Spanned::new("(\\r)")).unwrap_err();

            match err {
                Err::Failure(Error::EscapedNonReservedCharacter(e)) => {
                    assert_eq!(*e, "\\");
                }
                Err::Incomplete(_) | Err::Error(_) | Err::Failure(_) => {
                    panic!("wrong error: {:?}", err)
                }
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
                Alternative::Text(_) => {
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
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
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

        #[allow(clippy::too_many_lines)]
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
                    assert_eq!(*s, "/");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
            }
        }

        #[rustfmt::skip]
        #[test]
        fn does_not_allow_alternation_with_empty_alternative_by_adjacent_left_parameter() {
            match expression(Spanned::new("{int}/x")).unwrap_err() {
                Err::Failure(Error::EmptyAlternation(s)) => {
                    assert_eq!(*s, "/");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => panic!("wrong error: {:?}", e),
            }
        }

        #[rustfmt::skip]
        #[test]
        fn does_not_allow_alternation_with_empty_alternative_by_adjacent_optional() {
            match expression(Spanned::new("three (brown)/black mice")).unwrap_err() {
                Err::Failure(Error::OnlyOptionalInAlternation(s)) => {
                    assert_eq!(*s, "(brown)/black");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => panic!("wrong error: {:?}", e),
            }
        }

        #[rustfmt::skip]
        #[test]
        fn does_not_allow_alternation_with_empty_alternative_by_adjacent_right_parameter() {
            match expression(Spanned::new("x/{int}")).unwrap_err() {
                Err::Failure(Error::EmptyAlternation(s)) => {
                    assert_eq!(*s, "/");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => panic!("wrong error: {:?}", e),
            }
        }

        #[test]
        fn does_not_allow_alternation_with_empty_alternative() {
            match expression(Spanned::new("three brown//black mice"))
                .unwrap_err()
            {
                Err::Failure(Error::EmptyAlternation(s)) => {
                    assert_eq!(*s, "/");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
            }
        }

        #[test]
        fn does_not_allow_empty_optional() {
            match expression(Spanned::new("three () mice")).unwrap_err() {
                Err::Failure(Error::EmptyOptional(s)) => {
                    assert_eq!(*s, "()");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
            }
        }

        #[test]
        fn does_not_allow_nested_optional() {
            match expression(Spanned::new("(a(b))")).unwrap_err() {
                Err::Failure(Error::NestedOptional(s)) => {
                    assert_eq!(*s, "(b)");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
            }
        }

        #[test]
        fn does_not_allow_optional_parameter_types() {
            match expression(Spanned::new("({int})")).unwrap_err() {
                Err::Failure(Error::ParameterInOptional(s)) => {
                    assert_eq!(*s, "{int}");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
            }
        }

        #[test]
        fn does_not_allow_parameter_name_with_reserved_characters() {
            match expression(Spanned::new("{(string)}")).unwrap_err() {
                Err::Failure(Error::OptionalInParameter(s)) => {
                    assert_eq!(*s, "(string)");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
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
                    assert_eq!(*s, "{");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
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
                    assert_eq!(*s, "{string}");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
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
                    assert_eq!(*s, "(");
                }
                e @ (Err::Incomplete(_) | Err::Error(_) | Err::Failure(_)) => {
                    panic!("wrong error: {:?}", e)
                }
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
                        Whitespace,
                        Text (
                            LocatedSpan {
                                offset: 10,
                                line: 1,
                                fragment: "and",
                                extra: ()
                            }
                        ),
                        Whitespace,
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
        fn matches_anonymous_parameter_type() {
            let ast =
                format!("{:?}", unwrap_parser(expression(Spanned::new("{}"))));
            eq(
                ast,
                r#"Expression (
                    [
                        Parameter (
                            Parameter (
                                LocatedSpan {
                                    offset: 1,
                                    line: 1,
                                    fragment: "",
                                    extra: ()
                                }
                            )
                        )
                    ]
                )"#,
            );
        }

        #[test]
        fn matches_doubly_escaped_parenthesis() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new(
                    "three \\(exceptionally) \\{string} mice"
                )))
            );
            eq(
                ast,
                r#"Expression (
                    [
                        Text (
                            LocatedSpan {
                                offset: 0,
                                line: 1,
                                fragment: "three",
                                extra: ()
                            }
                        ),
                        Whitespace,
                        Text (
                            LocatedSpan {
                                offset: 6,
                                line: 1,
                                fragment: "\\(exceptionally)",
                                extra: ()
                            }
                        ),
                        Whitespace,
                        Text (
                            LocatedSpan {
                                offset: 23,
                                line: 1,
                                fragment: "\\{string}",
                                extra: ()
                            }
                        ),
                        Whitespace,
                        Text (
                            LocatedSpan {
                                offset: 33,
                                line: 1,
                                fragment: "mice",
                                extra: ()
                            }
                        )
                    ]
                )"#,
            );
        }

        #[test]
        fn matches_doubly_escaped_slash() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new("12\\\\/2020")))
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
                                                fragment: "12\\\\",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 5,
                                                line: 1,
                                                fragment: "2020",
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
        fn matches_optional_before_alternation() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new(
                    "three (brown )mice/rats"
                )))
            );
            eq(
                ast,
                r#"Expression (
                    [
                        Text (
                            LocatedSpan {
                                offset: 0,
                                line: 1,
                                fragment: "three",
                                extra: ()
                            }
                        ),
                        Whitespace,
                        Alternation (
                            Alternation (
                                [
                                    [
                                        Optional (
                                            Optional (
                                                LocatedSpan {
                                                    offset: 7,
                                                    line: 1,
                                                    fragment: "brown",
                                                    extra: ()
                                                }
                                            )
                                        ),
                                        Text (
                                            LocatedSpan {
                                                offset: 14,
                                                line: 1,
                                                fragment: "mice",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 19,
                                                line: 1,
                                                fragment: "rats",
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
        fn matches_optional_in_alternation() {
            let ast = format!(
                "{:?}",
                unwrap_parser(expression(Spanned::new(
                    "{int} rat(s)/mouse/mice"
                )))
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
                        Whitespace,
                        Alternation (
                            Alternation (
                                [
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 6,
                                                line: 1,
                                                fragment: "rat",
                                                extra: ()
                                            }
                                        ),
                                        Optional (
                                            Optional (
                                                LocatedSpan {
                                                    offset: 10,
                                                    line: 1,
                                                    fragment: "s",
                                                    extra: ()
                                                }
                                            )
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 13,
                                                line: 1,
                                                fragment: "mouse",
                                                extra: ()
                                            }
                                        )
                                    ],
                                    [
                                        Text (
                                            LocatedSpan {
                                                offset: 19,
                                                line: 1,
                                                fragment: "mice",
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
        fn empty() {
            let ast =
                format!("{:?}", unwrap_parser(expression(Spanned::new(""))));
            eq(ast, r#"Expression([])"#);
        }
    }
}
