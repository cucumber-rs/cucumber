// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use cute_custom_default::CustomDefault;

use gherkin::StepType;

use crate::World;
use crate::collection::StepsCollection;
use crate::runner::{BasicStepFn, RegexStepFn};

#[derive(CustomDefault)]
pub struct Steps<W: World> {
    pub(crate) steps: StepsCollection<W>,
}

impl<W: World> Steps<W> {
    pub fn new() -> Steps<W> {
        Steps { steps: StepsCollection::default() }
    }

    pub fn insert(&mut self, ty: StepType, name: &'static str, test_fn: BasicStepFn<W>) -> &mut Self {
        self.steps.insert_basic(ty, name, test_fn);
        self
    }

    pub fn insert_regex(&mut self, ty: StepType, name: &'static str, test_fn: RegexStepFn<W>) -> &mut Self {
        let regex = regex::Regex::new(name)
            .unwrap_or_else(|_| panic!("`{}` is not a valid regular expression", name));
        self.steps.insert_regex(ty, regex, test_fn);
        self
    }

    pub fn given(&mut self, name: &'static str, test_fn: BasicStepFn<W>) -> &mut Self {
        self.insert(StepType::Given, name, test_fn.into())
    }

    pub fn when(&mut self, name: &'static str, test_fn: BasicStepFn<W>) -> &mut Self {
        self.insert(StepType::When, name, test_fn)
    }

    pub fn then(&mut self, name: &'static str, test_fn: BasicStepFn<W>) -> &mut Self {
        self.insert(StepType::Then, name, test_fn)
    }

    pub fn given_regex(&mut self, name: &'static str, test_fn: RegexStepFn<W>) -> &mut Self {
        self.insert_regex(StepType::Given, name, test_fn)
    }

    pub fn when_regex(&mut self, name: &'static str, test_fn: RegexStepFn<W>) -> &mut Self {
        self.insert_regex(StepType::When, name, test_fn)
    }

    pub fn then_regex(&mut self, name: &'static str, test_fn: RegexStepFn<W>) -> &mut Self {
        self.insert_regex(StepType::Then, name, test_fn)
    }

    pub fn append(&mut self, other: Steps<W>) {
        self.steps.append(other.steps);
    }
}
