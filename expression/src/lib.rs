mod ast;
mod combinator;
pub mod parse;

#[doc(inline)]
pub use self::{
    ast::{
        Alternation, Alternative, Expression, Optional, SingleExpr, Spanned,
    },
    parse::Error,
};
