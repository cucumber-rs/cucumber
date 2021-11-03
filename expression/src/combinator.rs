use std::ops::RangeFrom;

use nom::{
    error::{ErrorKind, ParseError},
    AsChar, Err, IResult, InputIter, InputLength, InputTake,
    InputTakeAtPosition, Offset, Parser, Slice,
};

/// Applies `map` to `parser` [`IResult`].
///
/// Intended to use like [`verify()`], but with ability to map error.
///
/// [`verify()`]: nom::combinator::verify()
pub(crate) fn and_then<I, O1, O2, E: ParseError<I>, F, H>(
    mut parser: F,
    map: H,
) -> impl FnMut(I) -> IResult<I, O2, E>
where
    F: Parser<I, O1, E>,
    H: Fn(O1) -> Result<O2, Err<E>>,
{
    move |input: I| {
        parser
            .parse(input)
            .and_then(|(rest, parsed)| map(parsed).map(|ok| (rest, ok)))
    }
}

/// Applies `map` to `parser` [`IResult`] in case it errored.
///
/// Can be used to converted to harden [`Error`] to [`Failure`].
///
/// [`Error`]: nom::Err::Error
/// [`Failure`]: nom::Err::Failure
/// [`verify()`]: nom::combinator::verify()
pub(crate) fn map_err<I, O1, E: ParseError<I>, F, G>(
    mut parser: F,
    map: G,
) -> impl FnMut(I) -> IResult<I, O1, E>
where
    F: Parser<I, O1, E>,
    G: FnOnce(Err<E>) -> Err<E> + Copy,
{
    move |input: I| parser.parse(input).map_err(map)
}

/// Differences from [`escaped()`]:
/// 1. If `normal` matched empty sequence, tries to matched escaped;
/// 2. If after previous step `escapable` didn't match anything, returns empty
///    sequence;
/// 3. Errors with [`ErrorKind::Escaped`] if `control_char` was followed by a
///    non-`escapable` `Input`.
///
/// [`escaped()`]: nom::bytes::complete::escaped()
pub(crate) fn escaped0<'a, Input: 'a, Error, F, G, O1, O2>(
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
                    if i2.input_len() == 0 {
                        return Ok((input.slice(input.input_len()..), input));
                    } else if i2.input_len() == current_len {
                        consumed_nothing = true;
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
                            Err(_) => {
                                return Err(Err::Error(
                                    Error::from_error_kind(
                                        i,
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

#[cfg(test)]
mod escaped0_spec {
    use nom::{
        bytes::complete::escaped,
        character::complete::{digit0, digit1, one_of},
        error::{Error, ErrorKind},
        Err, IResult,
    };

    use super::escaped0;

    /// Type used to compare behaviour of [`escaped`] and [`escaped0`].
    ///
    /// Tuple is constructed from following parsers results:
    /// - [`escaped0`]`(`[`digit0`]`, '\\', `[`one_of`]`(r#""n\"#))`
    /// - [`escaped0`]`(`[`digit1`]`, '\\', `[`one_of`]`(r#""n\"#))`
    /// - [`escaped`]`(`[`digit0`]`, '\\', `[`one_of`]`(r#""n\"#))`
    /// - [`escaped`]`(`[`digit1`]`, '\\', `[`one_of`]`(r#""n\"#))`
    type TestResult<'s> = (
        IResult<&'s str, &'s str>,
        IResult<&'s str, &'s str>,
        IResult<&'s str, &'s str>,
        IResult<&'s str, &'s str>,
    );

    /// Produces [`TestResult`] from `input`.
    fn get_result(input: &str) -> TestResult {
        (
            escaped0(digit0, '\\', one_of(r#""n\"#))(input),
            escaped0(digit1, '\\', one_of(r#""n\"#))(input),
            escaped(digit0, '\\', one_of(r#""n\"#))(input),
            escaped(digit1, '\\', one_of(r#""n\"#))(input),
        )
    }

    #[test]
    fn matches_empty() {
        assert_eq!(
            get_result(""),
            (Ok(("", "")), Ok(("", "")), Ok(("", "")), Ok(("", ""))),
        );
    }

    #[test]
    fn matches_normal() {
        assert_eq!(
            get_result("123;"),
            (
                Ok((";", "123")),
                Ok((";", "123")),
                Ok((";", "123")),
                Ok((";", "123"))
            ),
        );
    }

    #[test]
    fn matches_only_escaped() {
        assert_eq!(
            get_result(r#"\n\";"#),
            (
                Ok((";", r#"\n\""#)),
                Ok((";", r#"\n\""#)),
                Ok((r#"\n\";"#, "")),
                Ok((";", r#"\n\""#)),
            ),
        );
    }

    #[test]
    fn matches_escaped_followed_by_normal() {
        assert_eq!(
            get_result(r#"\n\"123;"#),
            (
                Ok((";", r#"\n\"123"#)),
                Ok((";", r#"\n\"123"#)),
                Ok((r#"\n\"123;"#, "")),
                Ok((";", r#"\n\"123"#)),
            ),
        );
    }

    #[test]
    fn matches_normal_followed_by_escaped() {
        assert_eq!(
            get_result(r#"123\n\";"#),
            (
                Ok((";", r#"123\n\""#)),
                Ok((";", r#"123\n\""#)),
                Ok((r#"\n\";"#, "123")),
                Ok((";", r#"123\n\""#)),
            ),
        );
    }

    #[test]
    fn matches_escaped_followed_by_normal_then_escaped() {
        assert_eq!(
            get_result(r#"\n\"123\n;"#),
            (
                Ok((";", r#"\n\"123\n"#)),
                Ok((";", r#"\n\"123\n"#)),
                Ok((r#"\n\"123\n;"#, "")),
                Ok((";", r#"\n\"123\n"#)),
            ),
        );
    }

    #[test]
    fn matches_normal_followed_by_escaped_then_normal() {
        assert_eq!(
            get_result(r#"123\n\"567;"#),
            (
                Ok((";", r#"123\n\"567"#)),
                Ok((";", r#"123\n\"567"#)),
                Ok((r#"\n\"567;"#, "123")),
                Ok((";", r#"123\n\"567"#)),
            ),
        );
    }

    #[test]
    fn errors_on_escaped_non_reserved() {
        assert_eq!(
            get_result(r#"\n\r"#),
            (
                Err(Err::Error(Error {
                    input: r#"\r"#,
                    code: ErrorKind::Escaped
                })),
                Err(Err::Error(Error {
                    input: r#"\r"#,
                    code: ErrorKind::Escaped
                })),
                Ok((r#"\n\r"#, "")),
                Err(Err::Error(Error {
                    input: r#"r"#,
                    code: ErrorKind::OneOf
                })),
            ),
        );
    }
}
