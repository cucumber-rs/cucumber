// Copyright (c) 2018  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(feature = "nightly", feature(set_stdio))]

pub extern crate gherkin_rust as gherkin;
pub extern crate globwalk;
pub extern crate regex;

use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{stderr, Read, Write};
use std::ops::Deref;
use std::path::PathBuf;

pub use gherkin::Scenario;
use gherkin::{Feature, Step, StepType};
use regex::Regex;

pub mod cli;
mod output;

pub use output::default::DefaultOutput;
use output::OutputVisitor;

mod panic_trap;
use panic_trap::{PanicDetails, PanicTrap};

pub trait World: Default {}

type HelperFn = fn(&Scenario) -> ();

#[derive(Debug, Clone)]
pub struct HashableRegex(pub Regex);

impl Hash for HashableRegex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

impl PartialEq for HashableRegex {
    fn eq(&self, other: &HashableRegex) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for HashableRegex {}

impl Deref for HashableRegex {
    type Target = Regex;

    fn deref(&self) -> &Regex {
        &self.0
    }
}

type TestFn<T> = fn(&mut T, &Step) -> ();
type TestRegexFn<T> = fn(&mut T, &[String], &Step) -> ();

pub struct TestCase<T: Default>(pub TestFn<T>);
pub struct RegexTestCase<T: Default>(pub TestRegexFn<T>);

type TestBag<T> = HashMap<&'static str, TestCase<T>>;
type RegexBag<T> = HashMap<HashableRegex, RegexTestCase<T>>;

#[derive(Default)]
pub struct Steps<T: Default> {
    pub given: TestBag<T>,
    pub when: TestBag<T>,
    pub then: TestBag<T>,
    pub regex: RegexSteps<T>,
}

#[derive(Default)]
pub struct RegexSteps<T: Default> {
    pub given: RegexBag<T>,
    pub when: RegexBag<T>,
    pub then: RegexBag<T>,
}

pub enum TestCaseType<'a, T>
where
    T: 'a + Default,
{
    Normal(&'a TestCase<T>),
    Regex(&'a RegexTestCase<T>, Vec<String>),
}

pub enum TestResult {
    MutexPoisoned,
    Skipped,
    Unimplemented,
    Pass,
    Fail(PanicDetails, Vec<u8>),
}

impl<T: Default> Steps<T> {
    fn test_bag_for(&self, ty: StepType) -> &TestBag<T> {
        match ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then,
        }
    }

    fn regex_bag_for(&self, ty: StepType) -> &RegexBag<T> {
        match ty {
            StepType::Given => &self.regex.given,
            StepType::When => &self.regex.when,
            StepType::Then => &self.regex.then,
        }
    }

    fn test_type<'a>(&'a self, step: &Step) -> Option<TestCaseType<'a, T>> {
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

    fn run_test(
        &self,
        world: &mut T,
        test_type: TestCaseType<'_, T>,
        step: &Step,
        suppress_output: bool,
    ) -> TestResult {
        let test_result = PanicTrap::run(suppress_output, move || match test_type {
            TestCaseType::Normal(t) => (t.0)(world, &step),
            TestCaseType::Regex(t, ref c) => (t.0)(world, c, &step),
        });

        match test_result.result {
            Ok(_) => TestResult::Pass,
            Err(panic_info) => {
                if panic_info.payload.ends_with("cucumber test skipped") {
                    TestResult::Skipped
                } else {
                    TestResult::Fail(panic_info, test_result.stdout)
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
        before_fns: &Option<&[HelperFn]>,
        after_fns: &Option<&[HelperFn]>,
        suppress_output: bool,
        output: &mut impl OutputVisitor,
    ) -> bool {
        output.visit_scenario(rule, &scenario);

        if let Some(before_fns) = before_fns {
            for f in before_fns.iter() {
                f(&scenario);
            }
        }

        let mut world = {
            let panic_trap = PanicTrap::run(suppress_output, T::default);
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
                    TestResult::Fail(_, _) => {
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

        if let Some(after_fns) = after_fns {
            for f in after_fns.iter() {
                f(&scenario);
            }
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
        before_fns: Option<&[HelperFn]>,
        after_fns: Option<&[HelperFn]>,
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

        is_success
    }

    pub fn run(
        &self,
        feature_files: Vec<PathBuf>,
        before_fns: Option<&[HelperFn]>,
        after_fns: Option<&[HelperFn]>,
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
macro_rules! cucumber {
    (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr,
        setup: $setupfn:expr,
        before: $beforefns:expr,
        after: $afterfns:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; Some($setupfn); Some($beforefns); Some($afterfns));
    };

    (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr,
        setup: $setupfn:expr,
        before: $beforefns:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; Some($setupfn); Some($beforefns); None);
    };

        (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr,
        setup: $setupfn:expr,
        after: $afterfns:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; Some($setupfn); None; Some($afterfns));
    };

    (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr,
        before: $beforefns:expr,
        after: $afterfns:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; None; Some($beforefns); Some($afterfns));
    };

    (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr,
        before: $beforefns:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; None; Some($beforefns); None);
    };

    (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr,
        after: $afterfns:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; None; None; Some($afterfns));
    };

    (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr,
        setup: $setupfn:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; Some($setupfn); None; None);
    };

    (
        features: $featurepath:tt,
        world: $worldtype:path,
        steps: $vec:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; None; None; None);
    };

    (
        @finish; $featurepath:tt; $worldtype:path; $vec:expr; $setupfn:expr; $beforefns:expr; $afterfns:expr
    ) => {
        #[allow(unused_imports)]
        fn main() {
            use std::path::Path;
            use std::process;
            use $crate::globwalk::{glob, GlobWalkerBuilder};
            use $crate::gherkin::Scenario;
            use $crate::{Steps, World, DefaultOutput};
            use $crate::cli::make_app;

            let options = match make_app() {
                Ok(v) => v,
                Err(e) => panic!(e)
            };

            let walker = match &options.feature {
                Some(v) => glob(v).expect("feature glob is invalid"),
                None => match Path::new($featurepath).canonicalize() {
                    Ok(p) => {
                        GlobWalkerBuilder::new(p, "*.feature")
                            .case_insensitive(true)
                            .build()
                            .expect("feature path is invalid")
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        eprintln!("There was an error parsing \"{}\"; aborting.", $featurepath);
                        process::exit(1);
                    }
                }
            }.into_iter();

            let mut feature_files = walker
                .filter_map(Result::ok)
                .map(|entry| entry.path().to_owned())
                .collect::<Vec<_>>();
            feature_files.sort();

            let tests = {
                let step_groups: Vec<Steps<$worldtype>> = $vec.iter().map(|f| f()).collect();
                let mut combined_steps = Steps::default();

                for step_group in step_groups.into_iter() {
                    combined_steps.given.extend(step_group.given);
                    combined_steps.when.extend(step_group.when);
                    combined_steps.then.extend(step_group.then);

                    combined_steps.regex.given.extend(step_group.regex.given);
                    combined_steps.regex.when.extend(step_group.regex.when);
                    combined_steps.regex.then.extend(step_group.regex.then);
                }

                combined_steps
            };

            let mut output = DefaultOutput::default();

            let setup_fn: Option<fn() -> ()> = $setupfn;
            let before_fns: Option<&[fn(&Scenario) -> ()]> = $beforefns;
            let after_fns: Option<&[fn(&Scenario) -> ()]> = $afterfns;

            match setup_fn {
                Some(f) => f(),
                None => {}
            };

            if !tests.run(feature_files, before_fns, after_fns, options, &mut output) {
                process::exit(1);
            }
        }
    }
}

#[macro_export]
macro_rules! skip {
    () => {
        unimplemented!("cucumber test skipped");
    };
}

#[macro_export]
macro_rules! steps {
    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt $body:expr;
    ) => {
        $tests.regex.$ty.insert(
            $crate::HashableRegex($crate::regex::Regex::new($name).expect(&format!("{} is a valid regex", $name))),
                $crate::RegexTestCase($body));
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.regex.$ty.insert(
            $crate::HashableRegex($crate::regex::Regex::new($name).expect(&format!("{} is a valid regex", $name))),
                $crate::RegexTestCase($body));

        steps!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt ($($arg_type:ty),*) $body:expr;
    ) => {
        $tests.regex.$ty.insert(
            $crate::HashableRegex($crate::regex::Regex::new($name).expect(&format!("{} is a valid regex", $name))),
                $crate::RegexTestCase(|world: &mut $worldtype, matches, step| {
                    let closure: Box<Fn(&mut $worldtype, $($arg_type,)* &$crate::gherkin::Step) -> ()> = Box::new($body);
                    let mut matches = matches.into_iter().enumerate();

                    closure(
                        world,
                        $({
                            let (index, match_) = matches.next().unwrap();
                            match_.parse::<$arg_type>().expect(&format!("Failed to parse {}th argument '{}' to type {}", index, match_, stringify!($arg_type)))
                        },)*
                        step
                    )
                }));
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt ($($arg_type:ty),*) $body:expr; $( $items:tt )*
    ) => {
        $tests.regex.$ty.insert(
            $crate::HashableRegex($crate::regex::Regex::new($name).expect(&format!("{} is a valid regex", $name))),
                $crate::RegexTestCase(|world: &mut $worldtype, matches, step| {
                    let closure: Box<Fn(&mut $worldtype, $($arg_type,)* &$crate::gherkin::Step) -> ()> = Box::new($body);
                    let mut matches = matches.into_iter().enumerate().skip(1);

                    closure(
                        world,
                        $({
                            let (index, match_) = matches.next().unwrap();
                            match_.parse::<$arg_type>().expect(&format!("Failed to parse {}th argument '{}' to type {}", index, match_, stringify!($arg_type)))
                        },)*
                        step
                    )
                }));

        steps!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident $name:tt $body:expr;
    ) => {
        $tests.$ty.insert($name, $crate::TestCase($body));
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.$ty.insert($name, $crate::TestCase($body));

        steps!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        $worldtype:path => { $( $items:tt )* }
    ) => {
        #[allow(unused_imports)]
        pub fn steps() -> $crate::Steps<$worldtype> {
            use std::path::Path;
            use std::process;

            let mut tests: $crate::Steps<$worldtype> = Default::default();
            steps!(@gather_steps, $worldtype, tests, $( $items )*);
            tests
        }
    };
}
