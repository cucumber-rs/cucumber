// Copyright (c) 2021  Brendan Molloy <brendan@bbqsrc.net>,
//                     Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                     Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Cucumber expression][1] [AST][2] definitions.
//!
//! [1]: https://github.com/cucumber/cucumber-expressions#readme
//! [2]: https://en.wikipedia.org/wiki/Abstract_syntax_tree

use derive_more::{AsRef, Deref, DerefMut};
use nom::{error::ErrorKind, Err, InputLength};
use nom_locate::LocatedSpan;

use crate::{parse, Error};

/// A set of meta information about the location of a token.
pub type Spanned<'s> = LocatedSpan<&'s str>;

/// [Cucumber expression][1].
///
/// See [`parse::expression()`] for detailed syntax and examples.
///
/// [1]: https://github.com/cucumber/cucumber-expressions#readme
#[derive(AsRef, Clone, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Expression<Input>(pub Vec<SingleExpression<Input>>);

impl<'s> TryFrom<&'s str> for Expression<Spanned<'s>> {
    type Error = Error<Spanned<'s>>;

    fn try_from(value: &'s str) -> Result<Self, Self::Error> {
        parse::expression(Spanned::new(value))
            .map_err(|e| match e {
                Err::Error(e) | Err::Failure(e) => e,
                Err::Incomplete(n) => Error::Needed(n),
            })
            .and_then(|(rest, parsed)| {
                if rest.is_empty() {
                    Ok(parsed)
                } else {
                    Err(Error::Other(rest, ErrorKind::Verify))
                }
            })
    }
}

impl<'s> Expression<Spanned<'s>> {
    /// Tries to `input` into [`Expression`].
    ///
    /// # Errors
    ///
    /// See [`Error`] for more details.
    pub fn parse<I: AsRef<str>>(
        input: &'s I,
    ) -> Result<Self, Error<Spanned<'s>>> {
        Self::try_from(input.as_ref())
    }
}

/// Building block of an [`Expression`].
///
/// See [`parse::single_expression()`] for detailed syntax and examples.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SingleExpression<Input> {
    /// [`Alternation`].
    Alternation(Alternation<Input>),

    /// [`Optional`].
    Optional(Optional<Input>),

    /// [`Parameter`].
    Parameter(Parameter<Input>),

    /// Text without whitespaces.
    Text(Input),

    /// Whitespaces are treated as special case to avoid lookaheads and
    /// lookbehinds described in the [`architecture`][1]. This allows parser to
    /// have `O(n)` complexity.
    ///
    /// [1]: https://bit.ly/3k8DfcW
    Whitespace,
}

/// Allows to match one of [`SingleAlternation`]s.
///
/// See [`parse::alternation()`] for detailed syntax and examples.
#[derive(AsRef, Clone, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Alternation<Input>(pub Vec<SingleAlternation<Input>>);

/// Building block an [`Alternation`].
pub type SingleAlternation<Input> = Vec<Alternative<Input>>;

impl<Input: InputLength> Alternation<Input> {
    /// Returns length of capture from `Input`.
    pub(crate) fn span_len(&self) -> usize {
        self.0
            .iter()
            .flatten()
            .map(|alt| match alt {
                Alternative::Text(t) => t.input_len(),
                Alternative::Optional(opt) => opt.input_len() + 2,
            })
            .sum::<usize>()
            + self.len()
            - 1
    }

    /// Indicates whether one of [`SingleAlternation`]s consists only from
    /// [`Optional`]s.
    pub(crate) fn contains_only_optional(&self) -> bool {
        for single_alt in &**self {
            if single_alt
                .iter()
                .all(|alt| matches!(alt, Alternative::Optional(_)))
            {
                return true;
            }
        }
        false
    }
}

/// Building block of a [`SingleAlternation`].
///
/// See [`parse::alternative()`] for detailed syntax and examples.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alternative<Input> {
    /// [`Optional`].
    Optional(Optional<Input>),

    /// Text.
    Text(Input),
}

/// Allows to match optional `Input`.
///
/// See [`parse::optional()`] for detailed syntax and examples.
#[derive(AsRef, Clone, Copy, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Optional<Input>(pub Input);

/// Allows to match some special `Input` descried by a [`Parameter`] name.
///
/// See [`parse::parameter()`] for detailed syntax and examples.
#[derive(AsRef, Clone, Copy, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Parameter<Input>(pub Input);
