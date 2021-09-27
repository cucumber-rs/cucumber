// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for terminal output.

use std::borrow::Cow;

use console::Style;

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
    pub fn ok(&self, input: impl Into<Cow<'static, str>>) -> Cow<'static, str> {
        if self.is_present {
            self.ok.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present colors `input` with [`Styles::skipped`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn skipped(
        &self,
        input: impl Into<Cow<'static, str>>,
    ) -> Cow<'static, str> {
        if self.is_present {
            self.skipped.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present colors `input` with [`Styles::err`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn err(
        &self,
        input: impl Into<Cow<'static, str>>,
    ) -> Cow<'static, str> {
        if self.is_present {
            self.err.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present colors `input` with [`Styles::header`] color or
    /// leaves "as is" otherwise.
    #[must_use]
    pub fn header(
        &self,
        input: impl Into<Cow<'static, str>>,
    ) -> Cow<'static, str> {
        if self.is_present {
            self.header.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }

    /// If terminal is present makes `input` __bold__ or leaves "as is"
    /// otherwise.
    #[must_use]
    pub fn bold(
        &self,
        input: impl Into<Cow<'static, str>>,
    ) -> Cow<'static, str> {
        if self.is_present {
            self.bold.apply_to(input.into()).to_string().into()
        } else {
            input.into()
        }
    }
}
