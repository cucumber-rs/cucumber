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

use clap::Clap;
use regex::Regex;

/// Run the tests, pet a dog!.
///
/// __WARNING__ ⚠️: This CLI exists only for backwards compatibility. In `v0.11`
///                 it will be completely reworked:
///                 [cucumber-rs/cucumber#134][1].
///
/// [1]: https://github.com/cucumber-rs/cucumber/issues/134
#[derive(Clap, Debug)]
#[clap(author = "Brendan Molloy <brendan@bbqsrc.net>,\n\
                 Ilya Solovyiov <ilya.solovyiov@gmail.com>,\n\
                 Kai Ren <tyranron@gmail.com>")]
pub struct Opt {
    /// Regex to select scenarios from.
    #[clap(short = 'e', long = "expression", name = "regex")]
    pub filter: Option<Regex>,

    /// __WARNING__ ⚠️: This option is deprecated and will be removed it later
    ///                 releases. For now it does nothing.
    #[clap(long)]
    pub nocapture: bool,

    /// __WARNING__ ⚠️: This option is deprecated and will be removed it later
    ///                 releases. For now it does nothing.
    #[clap(long)]
    pub debug: bool,
}
