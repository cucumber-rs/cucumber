// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::BTreeMap;

use cute_custom_default::CustomDefault;
use regex::Regex;

use crate::regex::HashableRegex;
use crate::runner::{StepFn, TestFunction};
use crate::World;
use gherkin::{Step, StepType};

#[derive(CustomDefault)]
struct StepMaps<W: World> {
    #[def_exp = "BTreeMap::new()"]
    basic: BTreeMap<&'static str, StepFn<W>>,
    #[def_exp = "BTreeMap::new()"]
    regex: BTreeMap<HashableRegex, StepFn<W>>,
}

#[derive(CustomDefault)]
pub(crate) struct StepsCollection<W: World> {
    given: StepMaps<W>,
    when: StepMaps<W>,
    then: StepMaps<W>,
}

impl<W: World> StepsCollection<W> {
    pub(crate) fn append(&mut self, mut other: StepsCollection<W>) {
        self.given.basic.append(&mut other.given.basic);
        self.when.basic.append(&mut other.when.basic);
        self.then.basic.append(&mut other.then.basic);
        self.given.regex.append(&mut other.given.regex);
        self.when.regex.append(&mut other.when.regex);
        self.then.regex.append(&mut other.then.regex);
    }

    pub(crate) fn insert_basic(&mut self, ty: StepType, name: &'static str, callback: StepFn<W>) {
        match ty {
            StepType::Given => self.given.basic.insert(name, callback),
            StepType::When => self.when.basic.insert(name, callback),
            StepType::Then => self.then.basic.insert(name, callback),
        };
    }

    pub(crate) fn insert_regex(&mut self, ty: StepType, regex: Regex, callback: StepFn<W>) {
        let name = HashableRegex(regex);

        match ty {
            StepType::Given => self.given.regex.insert(name, callback),
            StepType::When => self.when.regex.insert(name, callback),
            StepType::Then => self.then.regex.insert(name, callback),
        };
    }

    pub(crate) fn resolve(&self, step: &Step) -> Option<TestFunction<W>> {
        // Attempt to find literal variant of steps first
        let test_fn = match step.ty {
            StepType::Given => self.given.basic.get(&*step.value),
            StepType::When => self.when.basic.get(&*step.value),
            StepType::Then => self.then.basic.get(&*step.value),
        };

        if let Some(function) = test_fn {
            return Some(TestFunction::from(function));
        }

        #[allow(clippy::mutable_key_type)]
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

            return Some(match *function {
                StepFn::Sync(x) => TestFunction::RegexSync(x, matches),
                StepFn::Async(x) => TestFunction::RegexAsync(x, matches),
            });
        }

        None
    }
}
