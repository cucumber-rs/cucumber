// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use cute_custom_default::CustomDefault;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

use super::{
    ArgsSyncTestFunction, LiteralSyncTestFunction, ArgsAsyncTestFunction, LiteralAsyncTestFunction, AsyncTestFunction, SyncTestFunction, TestFunction, TestPayload,
};
use crate::{hashable_regex::HashableRegex, Step, StepType, World};

#[derive(CustomDefault)]
struct StepMaps<W: World> {
    #[def_exp = "HashMap::new()"]
    literals: HashMap<&'static str, Arc<TestFunction<W>>>,
    #[def_exp = "HashMap::new()"]
    regex: HashMap<HashableRegex, Arc<TestFunction<W>>>,
}

#[derive(CustomDefault)]
pub(crate) struct StepsCollection<W: World> {
    given: StepMaps<W>,
    when: StepMaps<W>,
    then: StepMaps<W>,
}

impl<W: World> StepsCollection<W> {
    pub(crate) fn insert_literal(
        &mut self,
        ty: StepType,
        name: &'static str,
        callback: LiteralSyncTestFunction<W>,
    ) {
        let callback = Arc::new(TestFunction::Sync(SyncTestFunction::WithoutArgs(callback)));

        match ty {
            StepType::Given => self.given.literals.insert(name, callback),
            StepType::When => self.when.literals.insert(name, callback),
            StepType::Then => self.then.literals.insert(name, callback),
        };
    }

    pub(crate) fn insert_regex(
        &mut self,
        ty: StepType,
        regex: Regex,
        callback: ArgsSyncTestFunction<W>,
    ) {
        let callback = Arc::new(TestFunction::Sync(SyncTestFunction::WithArgs(callback)));
        let name = HashableRegex(regex);

        match ty {
            StepType::Given => self.given.regex.insert(name, callback),
            StepType::When => self.when.regex.insert(name, callback),
            StepType::Then => self.then.regex.insert(name, callback),
        };
    }

    pub(crate) fn insert_async_literal(
        &mut self,
        ty: StepType,
        name: &'static str,
        callback: LiteralAsyncTestFunction<W>,
    ) {
        let callback = Arc::new(TestFunction::Async(AsyncTestFunction::WithoutArgs(callback)));

        match ty {
            StepType::Given => self.given.literals.insert(name, callback),
            StepType::When => self.when.literals.insert(name, callback),
            StepType::Then => self.then.literals.insert(name, callback),
        };
    }

    pub(crate) fn insert_async_regex(
        &mut self,
        ty: StepType,
        regex: Regex,
        callback: ArgsAsyncTestFunction<W>,
    ) {
        let callback = Arc::new(TestFunction::Async(AsyncTestFunction::WithArgs(callback)));
        let name = HashableRegex(regex);

        match ty {
            StepType::Given => self.given.regex.insert(name, callback),
            StepType::When => self.when.regex.insert(name, callback),
            StepType::Then => self.then.regex.insert(name, callback),
        };
    }

    pub(crate) fn resolve(&self, step: &Step) -> Option<TestPayload<W>> {
        // Attempt to find literal variant of steps first
        let test_fn = match step.ty {
            StepType::Given => self.given.literals.get(&*step.value),
            StepType::When => self.when.literals.get(&*step.value),
            StepType::Then => self.then.literals.get(&*step.value),
        };

        match test_fn {
            Some(function) => {
                return Some(TestPayload {
                    function: Arc::clone(function),
                    payload: vec![],
                })
            }
            None => {}
        };

        let regex_map = match step.ty {
            StepType::Given => &self.given.regex,
            StepType::When => &self.when.regex,
            StepType::Then => &self.then.regex,
        };

        // Then attempt to find a regex variant of that test
        if let Some((regex, function)) = regex_map
            .iter()
            .find(|(regex, _)| regex.is_match(&step.value))
        {
            let matches = regex
                .0
                .captures(&step.value)
                .unwrap()
                .iter()
                .map(|match_| {
                    match_
                        .map(|match_| match_.as_str().to_owned())
                        .unwrap_or_default()
                })
                .collect();

            return Some(TestPayload {
                function: Arc::clone(function),
                payload: matches,
            });
        }

        None
    }
}
