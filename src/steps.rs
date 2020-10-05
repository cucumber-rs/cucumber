// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::rc::Rc;

use cute_custom_default::CustomDefault;

use crate::collection::StepsCollection;
use crate::runner::{BasicStepFn, RegexStepFn};
use crate::{TestError, World};
use futures::TryFutureExt;
use gherkin::StepType;

#[derive(CustomDefault)]
pub struct Steps<W: World> {
    pub(crate) steps: StepsCollection<W>,
}

impl<W: World> Steps<W> {
    pub fn new() -> Steps<W> {
        Steps {
            steps: StepsCollection::default(),
        }
    }

    fn insert_async(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: BasicStepFn<W>,
    ) -> &mut Self {
        self.steps.insert_basic(ty, name, test_fn);
        self
    }

    fn insert_sync(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: fn(W, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        use futures::future::FutureExt;

        let test_fn = std::rc::Rc::new(move |world: W, step| {
            // let test_fn = Rc::clone(&test_fn);
            std::panic::AssertUnwindSafe(async move { (test_fn)(world, step) })
                .catch_unwind()
                .map_err(TestError::PanicError)
                .boxed_local()
        });
        self.steps.insert_basic(ty, name, test_fn);
        self
    }

    fn insert_regex_async(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: RegexStepFn<W>,
    ) -> &mut Self {
        let regex = regex::Regex::new(name)
            .unwrap_or_else(|_| panic!("`{}` is not a valid regular expression", name));
        self.steps.insert_regex(ty, regex, test_fn);
        self
    }

    fn insert_regex_sync(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: fn(W, Vec<String>, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        use futures::future::FutureExt;
        let regex = regex::Regex::new(name)
            .unwrap_or_else(|_| panic!("`{}` is not a valid regular expression", name));

        let test_fn = std::rc::Rc::new(move |world: W, matches, step| {
            // let test_fn = Rc::clone(&test_fn);
            std::panic::AssertUnwindSafe(async move { (test_fn)(world, matches, step) })
                .catch_unwind()
                .map_err(TestError::PanicError)
                .boxed_local()
        });
        self.steps.insert_regex(ty, regex, test_fn);
        self
    }

    pub fn given_async(&mut self, name: &'static str, test_fn: BasicStepFn<W>) -> &mut Self {
        self.insert_async(StepType::Given, name, test_fn)
    }

    pub fn when_async(&mut self, name: &'static str, test_fn: BasicStepFn<W>) -> &mut Self {
        self.insert_async(StepType::When, name, test_fn)
    }

    pub fn then_async(&mut self, name: &'static str, test_fn: BasicStepFn<W>) -> &mut Self {
        self.insert_async(StepType::Then, name, test_fn)
    }

    pub fn given(
        &mut self,
        name: &'static str,
        test_fn: fn(W, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        self.insert_sync(StepType::Given, name, test_fn)
    }

    pub fn when(
        &mut self,
        name: &'static str,
        test_fn: fn(W, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        self.insert_sync(StepType::When, name, test_fn)
    }

    pub fn then(
        &mut self,
        name: &'static str,
        test_fn: fn(W, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        self.insert_sync(StepType::Then, name, test_fn)
    }

    pub fn given_regex_async(&mut self, name: &'static str, test_fn: RegexStepFn<W>) -> &mut Self {
        self.insert_regex_async(StepType::Given, name, test_fn)
    }

    pub fn when_regex_async(&mut self, name: &'static str, test_fn: RegexStepFn<W>) -> &mut Self {
        self.insert_regex_async(StepType::When, name, test_fn)
    }

    pub fn then_regex_async(&mut self, name: &'static str, test_fn: RegexStepFn<W>) -> &mut Self {
        self.insert_regex_async(StepType::Then, name, test_fn)
    }

    pub fn given_regex(
        &mut self,
        name: &'static str,
        test_fn: fn(W, Vec<String>, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        self.insert_regex_sync(StepType::Given, name, test_fn)
    }

    pub fn when_regex(
        &mut self,
        name: &'static str,
        test_fn: fn(W, Vec<String>, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        self.insert_regex_sync(StepType::When, name, test_fn)
    }

    pub fn then_regex(
        &mut self,
        name: &'static str,
        test_fn: fn(W, Vec<String>, Rc<gherkin::Step>) -> W,
    ) -> &mut Self {
        self.insert_regex_sync(StepType::Then, name, test_fn)
    }

    pub(crate) fn append(&mut self, other: Steps<W>) {
        self.steps.append(other.steps);
    }
}
