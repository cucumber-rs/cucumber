mod ast;
mod combinator;
mod parse;

#[doc(inline)]
pub use self::{
    ast::{
        Alternation, Alternative, Expression, Optional, SingleExpr, Spanned,
    },
    parse::Error,
};
