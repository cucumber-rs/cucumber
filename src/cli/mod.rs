// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for composing CLI options.
//!
//! The main thing in this module is [`Opts`], which compose all the strongly
//! typed CLI options from [`Parser`], [`Runner`] and [`Writer`], and provide
//! filtering based on [`Regex`] or [tag expressions][1].
//!
//! The idea behind this is that [`Parser`], [`Runner`] and/or [`Writer`] may
//! want to introduce their own CLI options to allow tweaking themselves, but we
//! still do want them combine in a single CLI and avoid any boilerplate burden.
//!
//! If the implementation doesn't need any CLI options, it may just use the
//! prepared [`cli::Empty`] stub.
//!
//! [`cli::Empty`]: self::Empty
//! [`Parser`]: crate::Parser
//! [`Runner`]: crate::Runner
//! [`Writer`]: crate::Writer
//! [1]: https://cucumber.io/docs/cucumber/api#tag-expressions

mod colored;
mod compose;
mod opts;

// Re-exports for backward compatibility and ease of use
pub use clap::{Args, Parser};

pub use colored::Colored;
pub use compose::{Compose, Empty};
pub use opts::Opts;

// Re-export Coloring from writer module for convenience
pub use crate::writer::Coloring;