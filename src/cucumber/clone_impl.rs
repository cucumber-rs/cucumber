//! Clone implementation for Cucumber executor.

use std::marker::PhantomData;

use crate::{
    Parser, Runner, World, Writer,
};

use super::core::Cucumber;

// Implemented manually to omit redundant `W: Clone` and `I: Clone` trait
// bounds, imposed by `#[derive(Clone)]`.
impl<W, P, I, R, Wr, Cli> Clone for Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Clone + Parser<I>,
    R: Clone + Runner<W>,
    Wr: Clone + Writer<W>,
    Cli: Clone + clap::Args,
    P::Cli: Clone,
    R::Cli: Clone,
    Wr::Cli: Clone,
{
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            runner: self.runner.clone(),
            writer: self.writer.clone(),
            cli: self.cli.clone(),
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}