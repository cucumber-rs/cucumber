// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use regex::Regex;

use super::{ArgsSyncTestFunction, LiteralSyncTestFunction, ArgsAsyncTestFunction, LiteralAsyncTestFunction, StepsCollection};
use crate::{StepType, World};

#[derive(Default)]
pub struct StepsBuilder<W>
where
    W: World,
{
    steps: StepsCollection<W>,
}

impl<W: World> StepsBuilder<W> {
    pub fn new() -> StepsBuilder<W> {
        StepsBuilder::default()
    }

    pub fn given(&mut self, name: &'static str, test_fn: LiteralSyncTestFunction<W>) -> &mut Self {
        self.add_literal(StepType::Given, name, test_fn);
        self
    }

    pub fn when(&mut self, name: &'static str, test_fn: LiteralSyncTestFunction<W>) -> &mut Self {
        self.add_literal(StepType::When, name, test_fn);
        self
    }

    pub fn then(&mut self, name: &'static str, test_fn: LiteralSyncTestFunction<W>) -> &mut Self {
        self.add_literal(StepType::Then, name, test_fn);
        self
    }

    pub fn given_regex(
        &mut self,
        regex: &'static str,
        test_fn: ArgsSyncTestFunction<W>,
    ) -> &mut Self {
        self.add_regex(StepType::Given, regex, test_fn);
        self
    }

    pub fn when_regex(
        &mut self,
        regex: &'static str,
        test_fn: ArgsSyncTestFunction<W>,
    ) -> &mut Self {
        self.add_regex(StepType::When, regex, test_fn);
        self
    }

    pub fn then_regex(
        &mut self,
        regex: &'static str,
        test_fn: ArgsSyncTestFunction<W>,
    ) -> &mut Self {
        self.add_regex(StepType::Then, regex, test_fn);
        self
    }


    pub fn given_async(&mut self, name: &'static str, test_fn: LiteralAsyncTestFunction<W>) -> &mut Self {
      self.add_async_literal(StepType::Given, name, test_fn);
      self
    }

    pub fn when_async(&mut self, name: &'static str, test_fn: LiteralAsyncTestFunction<W>) -> &mut Self {
        self.add_async_literal(StepType::When, name, test_fn);
        self
    }

    pub fn then_async(&mut self, name: &'static str, test_fn: LiteralAsyncTestFunction<W>) -> &mut Self {
        self.add_async_literal(StepType::Then, name, test_fn);
        self
    }

    pub fn given_async_regex(
        &mut self,
        regex: &'static str,
        test_fn: ArgsAsyncTestFunction<W>,
    ) -> &mut Self {
        self.add_async_regex(StepType::Given, regex, test_fn);
        self
    }

    pub fn when_async_regex(
        &mut self,
        regex: &'static str,
        test_fn: ArgsAsyncTestFunction<W>,
    ) -> &mut Self {
        self.add_async_regex(StepType::When, regex, test_fn);
        self
    }

    pub fn then_async_regex(
        &mut self,
        regex: &'static str,
        test_fn: ArgsAsyncTestFunction<W>,
    ) -> &mut Self {
        self.add_async_regex(StepType::Then, regex, test_fn);
        self
    }

    pub fn add_literal(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: LiteralSyncTestFunction<W>,
    ) -> &mut Self {
        self.steps.insert_literal(ty, name, test_fn);
        self
    }

    pub fn add_regex(
        &mut self,
        ty: StepType,
        regex: &str,
        test_fn: ArgsSyncTestFunction<W>,
    ) -> &mut Self {
        let regex = Regex::new(regex)
            .unwrap_or_else(|_| panic!("`{}` is not a valid regular expression", regex));
        self.steps.insert_regex(ty, regex, test_fn);

        self
    }

    pub fn add_async_literal(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: LiteralAsyncTestFunction<W>,
    ) -> &mut Self {
        self.steps.insert_async_literal(ty, name, test_fn);
        self
    }

    pub fn add_async_regex(
        &mut self,
        ty: StepType,
        regex: &str,
        test_fn: ArgsAsyncTestFunction<W>,
    ) -> &mut Self {
        let regex = Regex::new(regex)
            .unwrap_or_else(|_| panic!("`{}` is not a valid regular expression", regex));
        self.steps.insert_async_regex(ty, regex, test_fn);

        self
    }

    pub fn build(self) -> super::Steps<W> {
        super::Steps { steps: self.steps }
    }
}
