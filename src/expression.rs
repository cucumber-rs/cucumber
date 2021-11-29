// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [`Parameter`](trait@Parameter) definition.

pub use cucumber_codegen::Parameter;

/// Custom [`cucumber_expressions`] parameter. Should be implemented with
/// [`Parameter`](macro@Parameter) derive macro.
pub trait Parameter {
    /// [`Regex`] to match this parameter. Shouldn't contain capture groups.
    /// Correctness is checked by the [`Parameter`](macro@Parameter) derive
    /// macro.
    ///
    /// [`Regex`]: regex::Regex
    const REGEX: &'static str;

    /// Name which this [`Parameter`] will be referenced by.
    const NAME: &'static str;
}
