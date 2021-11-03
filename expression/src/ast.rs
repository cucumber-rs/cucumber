use std::option::Option as StdOption;

use nom_locate::LocatedSpan;

use crate::parse::Error;

pub type Spanned<'s> = LocatedSpan<&'s str>;

#[derive(Debug, Eq, PartialEq)]
pub struct Expression<'s>(pub(crate) Vec<SingleExpr<'s>>);

#[derive(Debug, Eq, PartialEq)]
pub enum SingleExpr<'s> {
    Alternation(Alternation<'s>),
    Optional(Optional<'s>),
    Parameter(Parameter<'s>),
    Text(Spanned<'s>),
    Space,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Alternation<'s>(pub(crate) Vec<Vec<Alternative<'s>>>);

impl<'s> Alternation<'s> {
    pub(crate) fn contains_only_optional(&self) -> StdOption<Error<'s>> {
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

#[derive(Debug, Eq, PartialEq)]
pub enum Alternative<'s> {
    Optional(Optional<'s>),
    Text(Spanned<'s>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct Optional<'s>(pub(crate) Vec<Option<'s>>);

impl<'s> Optional<'s> {
    pub(crate) fn span_len(&self) -> usize {
        self.0
            .iter()
            .map(|opt| match opt {
                Option::Optional(opt) => opt.span_len(),
                Option::Text(text) => text.len() + 2,
            })
            .sum::<usize>()
    }

    pub(crate) fn first_span(&self) -> Spanned<'s> {
        if let Some(opt) = self.0.first() {
            match opt {
                Option::Optional(opt) => opt.first_span(),
                Option::Text(text) => *text,
            }
        } else {
            panic!("");
        }
    }

    pub(crate) fn can_be_simplified(&self) -> StdOption<Spanned<'s>> {
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

#[derive(Debug, Eq, PartialEq)]
pub enum Option<'s> {
    Optional(Optional<'s>),
    Text(Spanned<'s>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct Parameter<'s>(pub(crate) Spanned<'s>);
