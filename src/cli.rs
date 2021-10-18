// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! CLI options.

use regex::Regex;

/// Run the tests, pet a dog!.
///
/// __WARNING__ ⚠️: This CLI exists only for backwards compatibility. In `0.11`
///                 it will be completely reworked:
///                 [cucumber-rs/cucumber#134][1].
///
/// [1]: https://github.com/cucumber-rs/cucumber/issues/134
#[derive(Debug, clap::Parser)]
pub struct Opts {
    /// Regex to select scenarios from.
    #[clap(short = 'e', long = "expression", name = "regex")]
    pub filter: Option<Regex>,

    /// __WARNING__ ⚠️: This option does nothing at the moment and is deprecated
    ///                 for removal in the next major release.
    ///                 Any output of step functions is not captured by default.
    #[clap(long)]
    pub nocapture: bool,

    /// __WARNING__ ⚠️: This option does nothing at the moment and is deprecated
    ///                 for removal in the next major release.
    #[clap(long)]
    pub debug: bool,
}
