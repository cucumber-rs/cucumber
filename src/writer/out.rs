// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for writing output.

use std::{borrow::Cow, io, str};

use console::Style;
use derive_more::{Deref, DerefMut, Display, From, Into};

/// [`Style`]s for terminal output.
#[derive(Debug)]
pub struct Styles {
    /// [`Style`] for rendering successful events.
    pub ok: Style,

    /// [`Style`] for rendering skipped events.
    pub skipped: Style,

    /// [`Style`] for rendering errors and failed events.
    pub err: Style,

    /// [`Style`] for rendering header.
    pub header: Style,

    /// [`Style`] for rendering __bold__.
    pub bold: Style,

    /// Indicates whether the terminal was detected.
    pub is_present: bool,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            ok: Style::new().green(),
            skipped: Style::new().cyan(),
            err: Style::new().red(),
            header: Style::new().blue(),
            bold: Style::new().bold(),
            is_present: atty::is(atty::Stream::Stdout)
                && console::colors_enabled(),
        }
    }
}

impl Styles {
    /// Creates new [`Styles`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
            .and_then(|_| self.write_str("\n"))
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
            .then(|| self.write_str(format!("\x1b[{}A", n)))
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
            .then(|| self.write_str(format!("\x1b[{}B", n)))
            .unwrap_or(Ok(()))
    }

    /// Writes a special sequence into this writer clearing the last `n` lines.
    ///
    /// # Errors
    ///
    /// If this writer fails to write a special sequence.
    fn clear_last_lines(&mut self, n: usize) -> io::Result<()> {
        self.move_cursor_up(n)?;
        for _ in 0..n {
            self.clear_line()?;
            self.move_cursor_down(1)?;
        }
        self.move_cursor_up(n)
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
