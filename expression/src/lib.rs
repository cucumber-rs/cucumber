mod ast;
mod combinator;
mod parse;

#[doc(inline)]
pub use self::{
    ast::{
        Alternation, Alternative, Expression, Option, Optional, SingleExpr,
        Spanned,
    },
    parse::Error,
};

// spec and test-data difference
// - matching/does-not-allow-nested-optional
//   parser/optional-containing-nested-optional.
//   So which is it?
