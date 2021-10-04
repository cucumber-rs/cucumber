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
use structopt::StructOpt;

/// CLI options.
///
/// > __WARNING__: This CLI exists only for backwards compatibility. In `v0.11`
/// >              it will be completely reworked:
///                [cucumber-rs/cucumber#134][1].
///
/// [1]: https://github.com/cucumber-rs/cucumber/issues/134
#[derive(Debug, StructOpt)]
#[structopt(
    about = "Run the tests, pet a dog!",
    author = "Brendan Molloy <brendan@bbqsrc.net>,\n\
              Ilya Solovyiov <ilya.solovyiov@gmail.com>,\n\
              Kai Ren <tyranron@gmail.com>"
)]
pub(crate) struct Opt {
    /// Regex to select scenarios from.
    #[structopt(short = "e", long = "expression", name = "regex")]
    pub(crate) filter: Option<Regex>,
}
