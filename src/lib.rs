// Copyright (c) 2018  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub extern crate gherkin;
pub extern crate globwalk;

pub mod cli;
mod hashable_regex;
mod output;
mod panic_trap;

use crate::cli::make_app;
use crate::globwalk::{glob, GlobWalkerBuilder};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{stderr, Read, Write};
use std::path::PathBuf;
use std::process;

use gherkin::Feature;
pub use gherkin::{Scenario, Step, StepType};
use regex::Regex;

use crate::hashable_regex::HashableRegex;
pub use crate::output::default::DefaultOutput;
pub use crate::output::OutputVisitor;
use crate::panic_trap::{PanicDetails, PanicTrap};

pub trait World: Default {}

type HelperFn = fn(&Scenario) -> ();

type TestFn<W> = fn(&mut W, &Step) -> ();
type RegexTestFn<W> = fn(&mut W, &[String], &Step) -> ();

type TestBag<W> = HashMap<&'static str, TestFn<W>>;
type RegexBag<W> = HashMap<HashableRegex, RegexTestFn<W>>;

#[derive(Default)]
pub struct Steps<W: World> {
    given: TestBag<W>,
    when: TestBag<W>,
    then: TestBag<W>,
    regex: RegexSteps<W>,
}

#[derive(Default)]
struct RegexSteps<W: World> {
    given: RegexBag<W>,
    when: RegexBag<W>,
    then: RegexBag<W>,
}

enum TestCaseType<'a, W: 'a + World> {
    Normal(&'a TestFn<W>),
    Regex(&'a RegexTestFn<W>, Vec<String>),
}

pub enum TestResult {
    Skipped,
    Unimplemented,
    Pass,
    Fail(PanicDetails, Vec<u8>, Vec<u8>),
}

#[derive(Default)]
pub struct StepsBuilder<W>
where
    W: World,
{
    steps: Steps<W>,
}

impl<W: World> StepsBuilder<W> {
    pub fn new() -> StepsBuilder<W> {
        StepsBuilder::default()
    }

    pub fn given(&mut self, name: &'static str, test_fn: TestFn<W>) -> &mut Self {
        self.add_normal(StepType::Given, name, test_fn);
        self
    }

    pub fn when(&mut self, name: &'static str, test_fn: TestFn<W>) -> &mut Self {
        self.add_normal(StepType::When, name, test_fn);
        self
    }

    pub fn then(&mut self, name: &'static str, test_fn: TestFn<W>) -> &mut Self {
        self.add_normal(StepType::Then, name, test_fn);
        self
    }

    pub fn given_regex(&mut self, regex: &'static str, test_fn: RegexTestFn<W>) -> &mut Self {
        self.add_regex(StepType::Given, regex, test_fn);
        self
    }

    pub fn when_regex(&mut self, regex: &'static str, test_fn: RegexTestFn<W>) -> &mut Self {
        self.add_regex(StepType::When, regex, test_fn);
        self
    }

    pub fn then_regex(&mut self, regex: &'static str, test_fn: RegexTestFn<W>) -> &mut Self {
        self.add_regex(StepType::Then, regex, test_fn);
        self
    }

    pub fn add_normal(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: TestFn<W>,
    ) -> &mut Self {
        self.steps.test_bag_mut_for(ty).insert(name, test_fn);
        self
    }

    pub fn add_regex(&mut self, ty: StepType, regex: &str, test_fn: RegexTestFn<W>) -> &mut Self {
        let regex = Regex::new(regex)
            .unwrap_or_else(|_| panic!("`{}` is not a valid regular expression", regex));

        self.steps
            .regex_bag_mut_for(ty)
            .insert(HashableRegex(regex), test_fn);

        self
    }

    pub fn build(self) -> Steps<W> {
        self.steps
    }
}

impl<W: World> Steps<W> {
    fn test_bag_for(&self, ty: StepType) -> &TestBag<W> {
        match ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then,
        }
    }

    fn test_bag_mut_for(&mut self, ty: StepType) -> &mut TestBag<W> {
        match ty {
            StepType::Given => &mut self.given,
            StepType::When => &mut self.when,
            StepType::Then => &mut self.then,
        }
    }

    fn regex_bag_for(&self, ty: StepType) -> &RegexBag<W> {
        match ty {
            StepType::Given => &self.regex.given,
            StepType::When => &self.regex.when,
            StepType::Then => &self.regex.then,
        }
    }

    fn regex_bag_mut_for(&mut self, ty: StepType) -> &mut RegexBag<W> {
        match ty {
            StepType::Given => &mut self.regex.given,
            StepType::When => &mut self.regex.when,
            StepType::Then => &mut self.regex.then,
        }
    }

    fn test_type<'a>(&'a self, step: &Step) -> Option<TestCaseType<'a, W>> {
        if let Some(t) = self.test_bag_for(step.ty).get(&*step.value) {
            return Some(TestCaseType::Normal(t));
        }

        if let Some((regex, t)) = self
            .regex_bag_for(step.ty)
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

            return Some(TestCaseType::Regex(t, matches));
        }

        None
    }

    pub fn combine(iter: impl Iterator<Item = Self>) -> Self {
        let mut combined = Self::default();

        for steps in iter {
            combined.given.extend(steps.given);
            combined.when.extend(steps.when);
            combined.then.extend(steps.then);

            combined.regex.given.extend(steps.regex.given);
            combined.regex.when.extend(steps.regex.when);
            combined.regex.then.extend(steps.regex.then);
        }

        combined
    }

    fn run_test(
        &self,
        world: &mut W,
        test_type: TestCaseType<'_, W>,
        step: &Step,
        suppress_output: bool,
    ) -> TestResult {
        let test_result = PanicTrap::run(suppress_output, || match test_type {
            TestCaseType::Normal(t) => t(world, &step),
            TestCaseType::Regex(t, ref c) => t(world, c, &step),
        });

        match test_result.result {
            Ok(_) => TestResult::Pass,
            Err(panic_info) => {
                if panic_info.payload.ends_with("cucumber test skipped") {
                    TestResult::Skipped
                } else {
                    TestResult::Fail(panic_info, test_result.stdout, test_result.stderr)
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn run_scenario(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        before_fns: &[HelperFn],
        after_fns: &[HelperFn],
        suppress_output: bool,
        output: &mut impl OutputVisitor,
    ) -> bool {
        output.visit_scenario(rule, &scenario);

        for f in before_fns.iter() {
            f(&scenario);
        }

        let mut world = {
            let panic_trap = PanicTrap::run(suppress_output, W::default);
            match panic_trap.result {
                Ok(v) => v,
                Err(panic_info) => {
                    eprintln!(
                        "Panic caught during world creation. Panic location: {}",
                        panic_info.location
                    );
                    if !panic_trap.stdout.is_empty() {
                        eprintln!("Captured output was:");
                        Write::write(&mut stderr(), &panic_trap.stdout).unwrap();
                    }
                    panic!(panic_info.payload);
                }
            }
        };

        let mut is_success = true;
        let mut is_skipping = false;

        let steps = feature
            .background
            .iter()
            .map(|bg| bg.steps.iter())
            .flatten()
            .chain(scenario.steps.iter());

        for step in steps {
            output.visit_step(rule, &scenario, &step);

            let test_type = match self.test_type(&step) {
                Some(v) => v,
                None => {
                    output.visit_step_result(rule, &scenario, &step, &TestResult::Unimplemented);
                    if !is_skipping {
                        is_skipping = true;
                        output.visit_scenario_skipped(rule, &scenario);
                    }
                    continue;
                }
            };

            if is_skipping {
                output.visit_step_result(rule, &scenario, &step, &TestResult::Skipped);
            } else {
                let result = self.run_test(&mut world, test_type, &step, suppress_output);
                output.visit_step_result(rule, &scenario, &step, &result);
                match result {
                    TestResult::Pass => {}
                    TestResult::Fail(_, _, _) => {
                        is_success = false;
                        is_skipping = true;
                    }
                    _ => {
                        is_skipping = true;
                        output.visit_scenario_skipped(rule, &scenario);
                    }
                };
            }
        }

        for f in after_fns.iter() {
            f(&scenario);
        }

        output.visit_scenario_end(rule, &scenario);

        is_success
    }

    #[allow(clippy::too_many_arguments)]
    fn run_scenarios(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenarios: &[gherkin::Scenario],
        before_fns: &[HelperFn],
        after_fns: &[HelperFn],
        options: &cli::CliOptions,
        output: &mut impl OutputVisitor,
    ) -> bool {
        let mut is_success = true;

        for scenario in scenarios {
            // If a tag is specified and the scenario does not have the tag, skip the test.
            let should_skip = match (&scenario.tags, &options.tag) {
                (Some(ref tags), Some(ref tag)) => !tags.contains(tag),
                _ => false,
            };

            if should_skip {
                continue;
            }

            match &scenario.examples {
                Some(examples) => {
                    for (i, row) in examples.table.rows.iter().enumerate() {
                        let steps = scenario
                            .steps
                            .iter()
                            .map(|step| {
                                let mut step = step.clone();
                                for (k, v) in examples.table.header.iter().zip(row.iter()) {
                                    step.value = step.value.replace(&format!("<{}>", k), &v);
                                    // Replace the values in the doc strings
                                    step.docstring =
                                        step.docstring.map(|x| x.replace(&format!("<{}>", k), &v));
                                    // TODO: also replace those in the table.
                                }
                                step
                            })
                            .collect();

                        // Replace example scenario name with example values
                        let mut scenario_name = scenario.name.clone();
                        for (k, v) in examples.table.header.iter().zip(row.iter()) {
                            scenario_name = scenario_name.replace(&format!("<{}>", k), &v);
                        }
                        // Graceful degradation
                        if scenario_name == scenario.name {
                            scenario_name = format!("{} {}", scenario.name, i);
                        }

                        let example = Scenario {
                            name: scenario_name,
                            steps,
                            examples: None,
                            tags: scenario.tags.clone(),
                            position: examples.table.position,
                        };

                        // If regex filter fails, skip the test.
                        if let Some(ref regex) = options.filter {
                            if !regex.is_match(&scenario.name) {
                                continue;
                            }
                        }

                        if !self.run_scenario(
                            &feature,
                            rule,
                            &example,
                            &before_fns,
                            &after_fns,
                            options.suppress_output,
                            output,
                        ) {
                            is_success = false;
                        }
                    }
                }
                None => {
                    // If regex filter fails, skip the test.
                    if let Some(ref regex) = options.filter {
                        if !regex.is_match(&scenario.name) {
                            continue;
                        }
                    }

                    if !self.run_scenario(
                        &feature,
                        rule,
                        &scenario,
                        &before_fns,
                        &after_fns,
                        options.suppress_output,
                        output,
                    ) {
                        is_success = false;
                    }
                }
            };
        }

        is_success
    }

    pub fn run(
        &self,
        feature_files: Vec<PathBuf>,
        before_fns: &[HelperFn],
        after_fns: &[HelperFn],
        options: cli::CliOptions,
        output: &mut impl OutputVisitor,
    ) -> bool {
        output.visit_start();

        let mut is_success = true;

        for path in feature_files {
            let mut file = File::open(&path).expect("file to open");
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).unwrap();

            let feature = match Feature::try_from(&*buffer) {
                Ok(v) => v,
                Err(e) => {
                    output.visit_feature_error(&path, &e);
                    is_success = false;
                    continue;
                }
            };

            output.visit_feature(&feature, &path);
            if !self.run_scenarios(
                &feature,
                None,
                &feature.scenarios,
                before_fns,
                after_fns,
                &options,
                output,
            ) {
                is_success = false;
            }

            for rule in &feature.rules {
                output.visit_rule(&rule);
                if !self.run_scenarios(
                    &feature,
                    Some(&rule),
                    &rule.scenarios,
                    before_fns,
                    after_fns,
                    &options,
                    output,
                ) {
                    is_success = false;
                }
                output.visit_rule_end(&rule);
            }
            output.visit_feature_end(&feature);
        }

        output.visit_finish();

        is_success
    }
}

#[doc(hidden)]
pub fn tag_rule_applies(scenario: &Scenario, rule: &str) -> bool {
    if let Some(ref tags) = &scenario.tags {
        let tags: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
        let rule_chunks = rule.split(' ');
        // TODO: implement a sane parser for this
        for rule in rule_chunks {
            if rule == "and" || rule == "or" {
                // TODO: implement handling for this
                continue;
            }

            if !tags.contains(&rule) {
                return false;
            }
        }

        true
    } else {
        true
    }
}

pub struct CucumberBuilder<W: World, O: OutputVisitor> {
    output: O,
    features: Vec<PathBuf>,
    setup: Option<fn() -> ()>,
    before: Vec<fn(&Scenario) -> ()>,
    after: Vec<fn(&Scenario) -> ()>,
    steps: Steps<W>,
    options: crate::cli::CliOptions,
}

impl<W: World, O: OutputVisitor> CucumberBuilder<W, O> {
    pub fn new(output: O) -> Self {
        CucumberBuilder {
            output,
            features: vec![],
            setup: None,
            before: vec![],
            after: vec![],
            steps: Steps::default(),
            options: crate::cli::CliOptions::default(),
        }
    }

    pub fn setup(&mut self, function: fn() -> ()) -> &mut Self {
        self.setup = Some(function);
        self
    }

    pub fn features(&mut self, features: Vec<PathBuf>) -> &mut Self {
        let mut features = features
            .iter()
            .map(|path| match path.canonicalize() {
                Ok(p) => GlobWalkerBuilder::new(p, "*.feature")
                    .case_insensitive(true)
                    .build()
                    .expect("feature path is invalid"),
                Err(e) => {
                    eprintln!("{}", e);
                    eprintln!("There was an error parsing {:?}; aborting.", path);
                    process::exit(1);
                }
            })
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path().to_owned())
            .collect::<Vec<_>>();
        features.sort();

        self.features = features;
        self
    }

    pub fn before(&mut self, functions: Vec<fn(&Scenario) -> ()>) -> &mut Self {
        self.before = functions;
        self
    }

    pub fn add_before(&mut self, function: fn(&Scenario) -> ()) -> &mut Self {
        self.before.push(function);
        self
    }

    pub fn after(&mut self, functions: Vec<fn(&Scenario) -> ()>) -> &mut Self {
        self.after = functions;
        self
    }

    pub fn add_after(&mut self, function: fn(&Scenario) -> ()) -> &mut Self {
        self.after.push(function);
        self
    }

    pub fn steps(&mut self, steps: Steps<W>) -> &mut Self {
        self.steps = steps;
        self
    }

    pub fn options(&mut self, options: crate::cli::CliOptions) -> &mut Self {
        self.options = options;
        self
    }

    pub fn run(mut self) -> bool {
        if let Some(feature) = self.options.feature.as_ref() {
            let features = glob(feature)
                .expect("feature glob is invalid")
                .filter_map(Result::ok)
                .map(|entry| entry.path().to_owned())
                .collect::<Vec<_>>();
            self.features(features);
        }

        if let Some(setup) = self.setup {
            setup();
        }

        self.steps.run(
            self.features,
            &self.before,
            &self.after,
            self.options,
            &mut self.output,
        )
    }

    pub fn command_line(mut self) -> bool {
        let options = make_app().unwrap();
        self.options(options);
        self.run()
    }
}


#[macro_export]
macro_rules! before {
    (
        $fnname:ident: $tagrule:tt => $scenariofn:expr
    ) => {
        fn $fnname(scenario: &$crate::Scenario) {
            let scenario_closure: fn(&$crate::Scenario) -> () = $scenariofn;
            let tag_rule: &str = $tagrule;

            // TODO check tags
            if $crate::tag_rule_applies(scenario, tag_rule) {
                scenario_closure(scenario);
            }
        }
    };

    (
        $fnname:ident => $scenariofn:expr
    ) => {
        before!($fnname: "" => $scenariofn);
    };
}

// This is just a remap of before.
#[macro_export]
macro_rules! after {
    (
        $fnname:ident: $tagrule:tt => $stepfn:expr
    ) => {
        before!($fnname: $tagrule => $stepfn);
    };

    (
        $fnname:ident => $scenariofn:expr
    ) => {
        before!($fnname: "" => $scenariofn);
    };
}

#[macro_export]
macro_rules! typed_regex {
    (
        $worldtype:path, ($($arg_type:ty),*) $body:expr
    ) => {
        |world: &mut $worldtype, matches, step| {
            let body: fn(&mut $worldtype, $($arg_type,)* &$crate::Step) -> () = $body;
            let mut matches = matches.into_iter().enumerate().skip(1);

            body(
                world,
                $({
                    let (index, match_) = matches.next().unwrap();
                    match_.parse::<$arg_type>().unwrap_or_else(|_| panic!("Failed to parse argument {} with value '{}' to type {}", index, match_, stringify!($arg_type)))
                },)*
                step
            )
        }
    };
}

#[macro_export]
macro_rules! skip {
    () => {
        unimplemented!("cucumber test skipped");
    };
}
