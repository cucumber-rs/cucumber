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
    pub(crate) fn contains_only_optional(&self) -> Option<Error<'s>> {
        for alt in &self.0 {
            if alt.len() == 1 {
                if let Some(Alternative::Optional(opt)) = alt.last() {
                    return Some(Error::OnlyOptionalInAlternation(**opt));
                }
            }
        }
        None
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
