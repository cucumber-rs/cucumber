use derive_more::{AsRef, Deref, DerefMut};
use nom_locate::LocatedSpan;

use crate::parse::Error;

pub type Spanned<'s> = LocatedSpan<&'s str>;

#[derive(AsRef, Clone, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Expression<'s>(pub Vec<SingleExpr<'s>>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SingleExpr<'s> {
    Alternation(Alternation<'s>),
    Optional(Optional<'s>),
    Parameter(Parameter<'s>),
    Text(Spanned<'s>),
    Space,
}

#[derive(AsRef, Clone, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Alternation<'s>(pub Vec<SingleAlternation<'s>>);

pub type SingleAlternation<'s> = Vec<Alternative<'s>>;

impl<'s> Alternation<'s> {
    pub(crate) fn span_len(&self) -> usize {
        self.0
            .iter()
            .flatten()
            .map(|alt| match alt {
                Alternative::Text(t) => t.len(),
                Alternative::Optional(opt) => opt.len() + 2,
            })
            .sum::<usize>()
            + self.len()
            - 1
    }

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alternative<'s> {
    Optional(Optional<'s>),
    Text(Spanned<'s>),
}

#[derive(AsRef, Clone, Copy, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Optional<'s>(pub Spanned<'s>);

#[derive(AsRef, Clone, Copy, Debug, Deref, DerefMut, Eq, PartialEq)]
pub struct Parameter<'s>(pub Spanned<'s>);
