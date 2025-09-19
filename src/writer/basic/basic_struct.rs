//! Core Basic writer struct and constructors.

use std::{fmt::Display, io};

use derive_more::with_trait::{Deref, DerefMut};

use crate::{
    writer::{self, Verbosity, out::{Styles, WriteStrExt as _}, Ext as _},
};

use super::cli::{Cli, Coloring};

/// Default [`Writer`] implementation outputting to an [`io::Write`] implementor
/// ([`io::Stdout`] by default).
///
/// Pretty-prints with colors if terminal was successfully detected, otherwise
/// has simple output. Useful for running tests with CI tools.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into
/// a [`writer::Normalize`], otherwise will produce output [`Event`]s in a
/// broken order.
///
/// [`Event`]: crate::Event
/// [`Normalized`]: writer::Normalized
/// [`Writer`]: crate::Writer
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct Basic<Out: io::Write = io::Stdout> {
    /// [`io::Write`] implementor to write the output into.
    #[deref]
    #[deref_mut]
    pub(super) output: Out,

    /// [`Styles`] for terminal output.
    pub(super) styles: Styles,

    /// Current indentation that events are outputted with.
    pub(super) indent: usize,

    /// Number of lines to clear.
    pub(super) lines_to_clear: usize,

    /// Buffer to be re-output after [`clear_last_lines_if_term_present()`][0].
    ///
    /// [0]: Self::clear_last_lines_if_term_present
    pub(super) re_output_after_clear: String,

    /// [`Verbosity`] of this [`Writer`].
    pub(super) verbosity: Verbosity,
}

impl Basic {
    /// Creates a new [`Normalized`] [`Basic`] [`Writer`] outputting to
    /// [`io::Stdout`].
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn stdout<W>() -> writer::Normalize<W, Self> {
        Self::new(io::stdout(), Coloring::Auto, Verbosity::Default)
    }
}

impl<Out: io::Write> Basic<Out> {
    /// Creates a new [`Normalized`] [`Basic`] [`Writer`] outputting to the
    /// given `output`.
    ///
    /// [`Normalized`]: writer::Normalize
    #[must_use]
    pub fn new<W>(
        output: Out,
        color: Coloring,
        verbosity: impl Into<Verbosity>,
    ) -> writer::Normalize<W, Self> {
        Self::raw(output, color, verbosity).normalized()
    }

    /// Creates a new non-[`Normalized`] [`Basic`] [`Writer`] outputting to the
    /// given `output`.
    ///
    /// Use it only if you know what you're doing. Otherwise, consider using
    /// [`Basic::new()`] which creates an already [`Normalized`] version of a
    /// [`Basic`] [`Writer`].
    ///
    /// [`Normalized`]: writer::Normalize
    #[must_use]
    pub fn raw(
        output: Out,
        color: Coloring,
        verbosity: impl Into<Verbosity>,
    ) -> Self {
        let mut basic = Self {
            output,
            styles: Styles::new(),
            indent: 0,
            lines_to_clear: 0,
            re_output_after_clear: String::new(),
            verbosity: verbosity.into(),
        };
        basic.apply_cli(Cli { verbose: u8::from(basic.verbosity) + 1, color });
        basic
    }

    /// Applies the given [`Cli`] options to this [`Basic`] [`Writer`].
    pub fn apply_cli(&mut self, cli: Cli) {
        match cli.verbose {
            0 => {}
            1 => self.verbosity = Verbosity::Default,
            2 => self.verbosity = Verbosity::ShowWorld,
            _ => self.verbosity = Verbosity::ShowWorldAndDocString,
        }
        self.styles.apply_coloring(cli.color);
    }

    /// Clears last `n` lines if [`Coloring`] is enabled.
    pub(super) fn clear_last_lines_if_term_present(&mut self) -> io::Result<()> {
        if self.styles.is_present && self.lines_to_clear > 0 {
            self.output.clear_last_lines(self.lines_to_clear)?;
            self.output.write_str(&self.re_output_after_clear)?;
            self.re_output_after_clear.clear();
            self.lines_to_clear = 0;
        }
        Ok(())
    }

    /// Outputs the parsing `error` encountered while parsing some [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    pub(super) fn parsing_failed(
        &mut self,
        error: impl Display,
    ) -> io::Result<()> {
        self.output
            .write_line(self.styles.err(format!("Failed to parse: {error}")))
    }
}

