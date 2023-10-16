// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for writing output.

use std::{
    borrow::Cow,
    io::{self, IsTerminal},
    mem, str,
};

use console::Style;
use derive_more::{Deref, DerefMut, Display, From, Into};

use super::Coloring;

/// [`Style`]s for terminal output.
#[derive(Clone, Debug)]
pub struct Styles {
    /// [`Style`] for rendering successful events.
    pub ok: Style,

    /// [`Style`] for rendering skipped events.
    pub skipped: Style,

    /// [`Style`] for rendering errors and failed events.
    pub err: Style,

    /// [`Style`] for rendering retried [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub retry: Style,

    /// [`Style`] for rendering header.
    pub header: Style,

    /// [`Style`] for rendering __bold__.
    pub bold: Style,

    /// [`Term`] width.
    ///
    /// [`Term`]: console::Term
    pub term_width: Option<u16>,

    /// Indicates whether the terminal was detected.
    pub is_present: bool,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            ok: Style::new().green(),
            skipped: Style::new().cyan(),
            err: Style::new().red(),
            retry: Style::new().magenta(),
            header: Style::new().blue(),
            bold: Style::new().bold(),
            term_width: console::Term::stdout().size_checked().map(|(_h, w)| w),
            is_present: io::stdout().is_terminal() && console::colors_enabled(),
        }
    }
}

impl Styles {
    /// Creates new [`Styles`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies the given [`Coloring`] to these [`Styles`].
    pub fn apply_coloring(&mut self, color: Coloring) {
        let is_present = match color {
            Coloring::Always => true,
            Coloring::Never => false,
            Coloring::Auto => return,
        };

        let this = mem::take(self);
        self.ok = this.ok.force_styling(is_present);
        self.skipped = this.skipped.force_styling(is_present);
        self.err = this.err.force_styling(is_present);
        self.retry = this.retry.force_styling(is_present);
        self.header = this.header.force_styling(is_present);
        self.bold = this.bold.force_styling(is_present);
        self.is_present = is_present;
    }

    /// Returns [`Styles`] with brighter colors.
    #[must_use]
    pub fn bright(&self) -> Self {
        Self {
            ok: self.ok.clone().bright(),
            skipped: self.skipped.clone().bright(),
            err: self.err.clone().bright(),
            retry: self.retry.clone().bright(),
            header: self.header.clone().bright(),
            bold: self.bold.clone().bright(),
            term_width: self.term_width,
            is_present: self.is_present,
        }
    }

    /// If terminal is present colors `input` with [`Styles::ok`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn ok<'a>(&self, input: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
        if self.is_present {
            self.ok.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present colors `input` with [`Styles::skipped`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn skipped<'a>(&self, input: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
        if self.is_present {
            self.skipped.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present colors `input` with [`Styles::err`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn err<'a>(&self, input: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
        if self.is_present {
            self.err.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present colors `input` with [`Styles::retry`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn retry<'a>(&self, input: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
        if self.is_present {
            self.retry.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present colors `input` with [`Styles::header`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn header<'a>(&self, input: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
        if self.is_present {
            self.header.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present makes `input` __bold__ or leaves "as is"
    /// otherwise.
    #[must_use]
    pub fn bold<'a>(&self, input: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
        if self.is_present {
            self.bold.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// Returns number of lines for the provided `s`tring, considering wrapping
    /// because of the [`Term`] width.
    ///
    /// [`Term`]: console::Term
    #[must_use]
    pub fn lines_count(&self, s: impl AsRef<str>) -> usize {
        // TODO: Remove, once `int_roundings` feature is stabilized:
        //       https://github.com/rust-lang/rust/issues/88581
        let div_ceil = |l, r| {
            let d = l / r;
            let rem = l % r;
            if rem > 0 && r > 0 {
                d + 1
            } else {
                d
            }
        };
        s.as_ref()
            .lines()
            .map(|l| {
                self.term_width
                    .map_or(1, |w| div_ceil(l.len(), usize::from(w)))
            })
            .sum()
    }
}

/// [`io::Write`] extension for easier manipulation with strings and special
/// sequences.
pub trait WriteStrExt: io::Write {
    /// Writes the given `string` into this writer.
    ///
    /// # Errors
    ///
    /// If this writer fails to write the given `string`.
    fn write_str(&mut self, string: impl AsRef<str>) -> io::Result<()> {
        self.write(string.as_ref().as_bytes()).map(drop)
    }

    /// Writes the given `string` into this writer followed by a newline.
    ///
    /// # Errors
    ///
    /// If this writer fails to write the given `string`.
    fn write_line(&mut self, string: impl AsRef<str>) -> io::Result<()> {
        self.write_str(string.as_ref())
            .and_then(|()| self.write_str("\n"))
            .map(drop)
    }

    /// Writes a special sequence into this writer moving a cursor up on `n`
    /// positions.
    ///
    /// # Errors
    ///
    /// If this writer fails to write a special sequence.
    fn move_cursor_up(&mut self, n: usize) -> io::Result<()> {
        (n > 0)
            .then(|| self.write_str(format!("\x1b[{n}A")))
            .unwrap_or(Ok(()))
    }

    /// Writes a special sequence into this writer moving a cursor down on `n`
    /// positions.
    ///
    /// # Errors
    ///
    /// If this writer fails to write a special sequence.
    fn move_cursor_down(&mut self, n: usize) -> io::Result<()> {
        (n > 0)
            .then(|| self.write_str(format!("\x1b[{n}B")))
            .unwrap_or(Ok(()))
    }

    /// Writes a special sequence into this writer clearing the last `n` lines.
    ///
    /// __NOTE:__ This method doesn't clear the current line, only the `n` lines
    ///           above it.
    ///
    /// # Errors
    ///
    /// If this writer fails to write a special sequence.
    fn clear_last_lines(&mut self, n: usize) -> io::Result<()> {
        for _ in 0..n {
            self.move_cursor_up(1)?;
            self.clear_line()?;
        }
        Ok(())
    }

    /// Writes a special sequence into this writer clearing the last line.
    ///
    /// # Errors
    ///
    /// If this writer fails to write a special sequence.
    fn clear_line(&mut self) -> io::Result<()> {
        self.write_str("\r\x1b[2K")
    }
}

impl<T: io::Write + ?Sized> WriteStrExt for T {}

/// [`String`] wrapper implementing [`io::Write`].
#[derive(
    Clone,
    Debug,
    Deref,
    DerefMut,
    Display,
    Eq,
    From,
    Hash,
    Into,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct WritableString(pub String);

impl io::Write for WritableString {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.push_str(
            str::from_utf8(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        );
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
