//! Parser configuration methods for Cucumber executor.

use std::{borrow::Cow, path::Path};

use crate::{
    Runner, World, Writer, parser,
};

use super::core::Cucumber;

impl<W, I, R, Wr, Cli> Cucumber<W, parser::Basic, I, R, Wr, Cli>
where
    W: World,
    R: Runner<W>,
    Wr: Writer<W>,
    Cli: clap::Args,
    I: AsRef<Path>,
{
    /// Sets the provided language of [`gherkin`] files.
    ///
    /// # Errors
    ///
    /// If the provided language isn't supported.
    pub fn language(
        mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<Self, parser::basic::UnsupportedLanguageError> {
        self.parser = self.parser.language(name)?;
        Ok(self)
    }
}