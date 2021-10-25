// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Extension of a [`TagOperation`].

use gherkin::tagexpr::TagOperation;
use sealed::sealed;

/// Extension of a [`TagOperation`] allowing to evaluate it.
#[sealed]
pub trait Ext {
    /// Evaluates this [`TagOperation`] for the given `tags`.
    #[must_use]
    fn eval(&self, tags: &[String]) -> bool;
}

#[sealed]
impl Ext for TagOperation {
    fn eval(&self, tags: &[String]) -> bool {
        match self {
            Self::And(l, r) => l.eval(tags) & r.eval(tags),
            Self::Or(l, r) => l.eval(tags) | r.eval(tags),
            Self::Not(t) => !t.eval(tags),
            Self::Tag(t) => tags.iter().any(|tag| tag == t),
        }
    }
}
