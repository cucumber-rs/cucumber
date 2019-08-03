// Copyright (c) 2018  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(async_await)]

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
use std::fmt;
use std::fs::File;
use std::io::{stderr, Read, Write};
use std::path::PathBuf;
use std::process;
use std::sync::{Arc, Mutex};

use gherkin::Feature;
pub use gherkin::{Scenario, Step, StepType};
use regex::Regex;

use crate::hashable_regex::HashableRegex;
pub use crate::output::default::DefaultOutput;
pub use crate::output::OutputVisitor;
use crate::panic_trap::{PanicDetails, PanicTrap};

use futures::future::{BoxFuture, Future, FutureExt};
use std::pin::Pin;

pub trait World: Default + Clone {}

type HelperFn = fn(&Scenario) -> ();

use std::panic::{AssertUnwindSafe, UnwindSafe};

type TestSyncFn<W> = fn(&mut W, &Step) -> ();
type RegexTestFn<W> = fn(&mut W, &[String], &Step) -> ();
type RegexTestSyncFn<W> = fn(&mut W, &[String], &Step) -> ();
type TestFn<W> = fn(W, Step) -> TestFuture;

pub struct TestFuture {
    future: BoxFuture<'static, ()>,
}

impl UnwindSafe for TestFuture {}

use futures::task::{Context, Poll};
use std::panic::catch_unwind;

use pin_utils::unsafe_pinned;
impl TestFuture {
    unsafe_pinned!(future: BoxFuture<'static, ()>);

    pub fn new(f: impl Future<Output = ()> + Send + 'static) -> Self {
        TestFuture { future: f.boxed() }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
impl Future for TestFuture {
    type Output = Result<(), Box<dyn std::any::Any + Send>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        catch_unwind(AssertUnwindSafe(|| self.future().poll(cx)))?.map(Ok)
    }
}

type TestAsyncBag<W> = HashMap<&'static str, TestFn<W>>;
type TestSyncBag<W> = HashMap<&'static str, TestSyncFn<W>>;
type RegexBag<W> = HashMap<HashableRegex, RegexTestFn<W>>;

#[derive(Default)]
pub struct Steps<W: World> {
    given: TestSyncBag<W>,
    when: TestSyncBag<W>,
    then: TestSyncBag<W>,
    regex: RegexSteps<W>,
    async_: AsyncSteps<W>,
}

#[derive(Default)]
struct RegexSteps<W: World> {
    given: RegexBag<W>,
    when: RegexBag<W>,
    then: RegexBag<W>,
}

impl<W: World> fmt::Debug for Steps<W> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Steps")
            .field("given", &self.given.keys())
            .field("when", &self.when.keys())
            .field("then", &self.then.keys())
            .field("regex", &self.regex)
            .field("async", &self.async_)
            .finish()
    }
}

impl<W: World> fmt::Debug for RegexSteps<W> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("RegexSteps")
            .field("given", &self.given.keys())
            .field("when", &self.when.keys())
            .field("then", &self.then.keys())
            .finish()
    }
}

impl<W: World> fmt::Debug for AsyncSteps<W> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("AsyncSteps")
            .field("given", &self.given.keys())
            .field("when", &self.when.keys())
            .field("then", &self.then.keys())
            .finish()
    }
}

#[derive(Default)]
struct AsyncSteps<W: World> {
    given: TestAsyncBag<W>,
    when: TestAsyncBag<W>,
    then: TestAsyncBag<W>,
}

impl<W: World> AsyncSteps<W> {
    pub fn len(&self) -> usize {
        self.given.len() + self.when.len() + self.then.len()
    }
}

impl<W: World> RegexSteps<W> {
    pub fn len(&self) -> usize {
        self.given.len() + self.when.len() + self.then.len()
    }
}

enum TestCaseType<'a, W: 'a + World> {
    Normal(&'a TestSyncFn<W>),
    Async(&'a TestFn<W>),
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

    pub fn add_normal_async(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: TestFn<W>,
    ) -> &mut Self {
        self.steps.async_bag_mut_for(ty).insert(name, test_fn);
        self
    }

    pub fn given_async(&mut self, name: &'static str, test_fn: TestFn<W>) -> &mut Self {
        self.add_normal_async(StepType::Given, name, test_fn);
        self
    }

    pub fn when_async(&mut self, name: &'static str, test_fn: TestFn<W>) -> &mut Self {
        self.add_normal_async(StepType::When, name, test_fn);
        self
    }

    pub fn then_async(&mut self, name: &'static str, test_fn: TestFn<W>) -> &mut Self {
        self.add_normal_async(StepType::Then, name, test_fn);
        self
    }

    pub fn given(&mut self, name: &'static str, test_fn: TestSyncFn<W>) -> &mut Self {
        self.add_normal(StepType::Given, name, test_fn);
        self
    }

    pub fn when(&mut self, name: &'static str, test_fn: TestSyncFn<W>) -> &mut Self {
        self.add_normal(StepType::When, name, test_fn);
        self
    }

    pub fn then(&mut self, name: &'static str, test_fn: TestSyncFn<W>) -> &mut Self {
        self.add_normal(StepType::Then, name, test_fn);
        self
    }

    pub fn given_regex(&mut self, regex: &'static str, test_fn: RegexTestSyncFn<W>) -> &mut Self {
        self.add_regex(StepType::Given, regex, test_fn);
        self
    }

    pub fn when_regex(&mut self, regex: &'static str, test_fn: RegexTestSyncFn<W>) -> &mut Self {
        self.add_regex(StepType::When, regex, test_fn);
        self
    }

    pub fn then_regex(&mut self, regex: &'static str, test_fn: RegexTestSyncFn<W>) -> &mut Self {
        self.add_regex(StepType::Then, regex, test_fn);
        self
    }

    pub fn add_normal(
        &mut self,
        ty: StepType,
        name: &'static str,
        test_fn: TestSyncFn<W>,
    ) -> &mut Self {
        self.steps.test_bag_mut_for(ty).insert(name, test_fn);
        self
    }

    pub fn add_regex(
        &mut self,
        ty: StepType,
        regex: &str,
        test_fn: RegexTestSyncFn<W>,
    ) -> &mut Self {
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

impl<W: World + Default> Steps<W> {
    pub fn len(&self) -> usize {
        self.given.len() + self.when.len() + self.then.len() + self.regex.len() + self.async_.len()
    }

    fn test_bag_for(&self, ty: StepType) -> &TestSyncBag<W> {
        match ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then,
        }
    }

    fn test_bag_mut_for(&mut self, ty: StepType) -> &mut TestSyncBag<W> {
        match ty {
            StepType::Given => &mut self.given,
            StepType::When => &mut self.when,
            StepType::Then => &mut self.then,
        }
    }

    fn async_bag_for(&self, ty: StepType) -> &TestAsyncBag<W> {
        match ty {
            StepType::Given => &self.async_.given,
            StepType::When => &self.async_.when,
            StepType::Then => &self.async_.then,
        }
    }

    fn async_bag_mut_for(&mut self, ty: StepType) -> &mut TestAsyncBag<W> {
        match ty {
            StepType::Given => &mut self.async_.given,
            StepType::When => &mut self.async_.when,
            StepType::Then => &mut self.async_.then,
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

        if let Some(t) = self.async_bag_for(step.ty).get(&*step.value) {
            return Some(TestCaseType::Async(t));
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

    pub fn append(&mut self, other: Self) {
        self.given.extend(other.given);
        self.when.extend(other.when);
        self.then.extend(other.then);

        self.regex.given.extend(other.regex.given);
        self.regex.when.extend(other.regex.when);
        self.regex.then.extend(other.regex.then);

        self.async_.given.extend(other.async_.given);
        self.async_.when.extend(other.async_.when);
        self.async_.then.extend(other.async_.then);
    }

    pub fn concat(iter: impl Iterator<Item = Self>) -> Self {
        iter.fold(Self::default(), |mut acc, steps| {
            acc.append(steps);
            acc
        })
    }

    async fn run_test<'f>(
        &self,
        world: &'f mut W,
        test_type: TestCaseType<'_, W>,
        step: Step,
        suppress_output: bool,
    ) -> TestResult {
        let test_result = match test_type {
            TestCaseType::Normal(t) => PanicTrap::run(suppress_output, || t(world, &step)),
            TestCaseType::Regex(t, ref c) => PanicTrap::run(suppress_output, || t(world, c, &step)),
            TestCaseType::Async(t) => {
                let unwindable = t(world.clone(), step.clone()).catch_unwind();
                let result = match unwindable.await {
                    Ok(unwind) => match unwind {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            let payload = if let Some(s) = e.downcast_ref::<String>() {
                                s.clone().to_string()
                            } else if let Some(s) = e.downcast_ref::<&str>() {
                                s.to_string()
                            } else {
                                "Opaque panic payload".to_owned()
                            };
                            Err(PanicDetails {
                                payload,
                                location: "<async>:0:0".into(),
                            })
                        }
                    },
                    Err(e) => Err(PanicDetails {
                        payload: "".into(),
                        location: "".into(),
                    }),
                };
                //     println!("OK: {:?}", &x);
                //     ()
                // })
                // .map_err(|e| {
                //     println!("ERR: {:?}", &e);
                //     PanicDetails { payload: "".into(), location: "".into() }
                // });
                // println!("RESULT: {:?}", &result);
                PanicTrap {
                    result,
                    stdout: vec![],
                    stderr: vec![],
                }
            }
        };

        // TestResult::Pass

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
    async fn run_scenario(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        before_fns: &[HelperFn],
        after_fns: &[HelperFn],
        suppress_output: bool,
        output: &impl OutputVisitor,
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

        let mut steps = vec![];

        if let Some(background) = feature.background.as_ref() {
            for step in background.steps.iter() {
                steps.push(step.to_owned());
            }
        }

        for step in scenario.steps.iter() {
            steps.push(step.clone());
        }

        for step in steps.into_iter() {
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
                let result = self
                    .run_test(&mut world, test_type, step.clone(), suppress_output)
                    .await;
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
    async fn run_scenarios(
        &self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenarios: &[gherkin::Scenario],
        before_fns: &[HelperFn],
        after_fns: &[HelperFn],
        options: &cli::CliOptions,
        output: &impl OutputVisitor,
    ) -> bool {
        let mut futures = vec![];

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

                        futures.push(self.run_scenario(
                            &feature,
                            rule,
                            &example,
                            &before_fns,
                            &after_fns,
                            options.suppress_output,
                            output.clone(),
                        ));
                    }
                }
                None => {
                    // If regex filter fails, skip the test.
                    if let Some(ref regex) = options.filter {
                        if !regex.is_match(&scenario.name) {
                            continue;
                        }
                    }

                    futures.push(self.run_scenario(
                        &feature,
                        rule,
                        &scenario,
                        &before_fns,
                        &after_fns,
                        options.suppress_output,
                        output.clone(),
                    ))
                }
            }
        }

        // Check if all are successful
        futures::future::join_all(futures)
            .await
            .into_iter()
            .all(|x| x)
    }

    pub async fn run(
        &self,
        feature_files: Vec<PathBuf>,
        before_fns: &[HelperFn],
        after_fns: &[HelperFn],
        options: cli::CliOptions,
        output: &impl OutputVisitor,
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
            if !self
                .run_scenarios(
                    &feature,
                    None,
                    &feature.scenarios,
                    before_fns,
                    after_fns,
                    &options,
                    output,
                )
                .await
            {
                is_success = false;
            }

            for rule in &feature.rules {
                output.visit_rule(&rule);
                if !self
                    .run_scenarios(
                        &feature,
                        Some(&rule),
                        &rule.scenarios,
                        before_fns,
                        after_fns,
                        &options,
                        output,
                    )
                    .await
                {
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

    pub fn setup(mut self, function: fn() -> ()) -> Self {
        self.setup = Some(function);
        self
    }

    pub fn features<P: AsRef<std::path::Path>>(mut self, features: Vec<P>) -> Self {
        let mut features = features
            .iter()
            .map(AsRef::as_ref)
            .map(|path| match path.canonicalize() {
                Ok(p) => GlobWalkerBuilder::new(p, "*.feature")
                    .case_insensitive(true)
                    .build()
                    .expect("feature path is invalid"),
                Err(e) => {
                    eprintln!("{}", e);
                    eprintln!("There was an error parsing {:?}; aborting.", path);
                    std::process::exit(1);
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

    pub fn before(mut self, functions: Vec<fn(&Scenario) -> ()>) -> Self {
        self.before = functions;
        self
    }

    pub fn add_before(mut self, function: fn(&Scenario) -> ()) -> Self {
        self.before.push(function);
        self
    }

    pub fn after(mut self, functions: Vec<fn(&Scenario) -> ()>) -> Self {
        self.after = functions;
        self
    }

    pub fn add_after(mut self, function: fn(&Scenario) -> ()) -> Self {
        self.after.push(function);
        self
    }

    pub fn steps(mut self, steps: Steps<W>) -> Self {
        self.steps = steps;
        self
    }

    pub fn add_steps(mut self, steps: Steps<W>) -> Self {
        self.steps.append(steps);
        self
    }

    pub fn options(mut self, options: crate::cli::CliOptions) -> Self {
        self.options = options;
        self
    }

    pub async fn run(mut self) -> bool {
        if self.features.len() == 0 {
            eprintln!("No features found; aborting.");
            return false;
        }

        if self.steps.len() == 0 {
            eprintln!("No steps found; aborting.");
            return false;
        }

        let this = if let Some(feature) = self.options.feature.as_ref() {
            let features = glob(feature)
                .expect("feature glob is invalid")
                .filter_map(Result::ok)
                .map(|entry| entry.path().to_owned())
                .collect::<Vec<_>>();
            self.features(features)
        } else {
            self
        };

        if let Some(ref setup) = this.setup {
            setup();
        }

        let steps = this.steps;

        steps
            .run(
                this.features,
                &this.before,
                &this.after,
                this.options,
                &this.output,
            )
            .await
    }

    pub async fn command_line(self) -> bool {
        let options = make_app().unwrap();
        self.options(options)
            .run()
            .await
    }
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
            use $crate::{CucumberBuilder, Scenario, Steps, DefaultOutput, OutputVisitor};

            let output = DefaultOutput::new();
            let instance = {
                let mut instance = CucumberBuilder::new(output);

                instance = instance
                    .features(vec![Path::new($featurepath).to_path_buf()])
                    .steps(Steps::concat($vec.iter().map(|f| f())));

                if let Some(setup) = $setupfn {
                    instance = instance.setup(setup);
                }

                let before_fns: Option<&[fn(&Scenario) -> ()]> = $beforefns;
                if let Some(before) = before_fns {
                    instance = instance.before(before.to_vec());
                }

                let after_fns: Option<&[fn(&Scenario) -> ()]> = $afterfns;
                if let Some(after) = after_fns {
                    instance = instance.after(after.to_vec());
                }

                instance
            };

            let res = futures::executor::block_on(instance.command_line());

            if !res {
                std::process::exit(1);
            }
        }
    }
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

#[macro_export]
macro_rules! steps {
    (
        @step_type given
    ) => {
        $crate::StepType::Given
    };

    (
        @step_type when
    ) => {
        $crate::StepType::When
    };

    (
        @step_type then
    ) => {
        $crate::StepType::Then
    };

    (
        @parse_matches $worldtype:path, ($($arg_type:ty),*) $body:expr
    ) => {
        $crate::typed_regex!($worldtype, ($($arg_type),*) $body)
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt $body:expr;
    ) => {
        $tests.add_regex(steps!(@step_type $ty), $name, $body);
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.add_regex(steps!(@step_type $ty), $name, $body);

        steps!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt ($($arg_type:ty),*) $body:expr;
    ) => {
        steps!(@gather_steps, $worldtype, $tests, $ty regex $name steps!(@parse_matches $worldtype, ($($arg_type),*) $body););
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt ($($arg_type:ty),*) $body:expr; $( $items:tt )*
    ) => {
        steps!(@gather_steps, $worldtype, $tests, $ty regex $name steps!(@parse_matches $worldtype, ($($arg_type),*) $body); $( $items )*);
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident $name:tt $body:expr;
    ) => {
        $tests.add_normal(steps!(@step_type $ty), $name, $body);
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.add_normal(steps!(@step_type $ty), $name, $body);

        steps!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        $worldtype:path => { $( $items:tt )* }
    ) => {
        #[allow(missing_docs)]
        pub fn steps() -> $crate::Steps<$worldtype> {
            let mut tests: $crate::StepsBuilder::<$worldtype> = $crate::StepsBuilder::new();
            steps!(@gather_steps, $worldtype, tests, $( $items )*);
            tests.build()
        }
    };
}
