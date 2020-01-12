// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod builder;
mod collection;

pub use self::builder::StepsBuilder;
pub(crate) use self::collection::StepsCollection;
use crate::panic_trap::{PanicDetails, PanicTrap};
use crate::{HelperFn, OutputVisitor, Step, World};

type LiteralSyncTestFunction<W> = fn(&mut W, &Step) -> ();
type ArgsSyncTestFunction<W> = fn(&mut W, &[String], &Step) -> ();

pub(crate) struct TestPayload<'a, W: World> {
    function: &'a TestFunction<W>,
    payload: Vec<String>,
}

enum SyncTestFunction<W> {
    WithArgs(fn(&mut W, &[String], &Step) -> ()),
    WithoutArgs(fn(&mut W, &Step) -> ()),
}

enum TestFunction<W> {
    Sync(SyncTestFunction<W>),
}

pub enum TestResult {
    Skipped,
    Unimplemented,
    Pass,
    Fail(PanicDetails, Vec<u8>, Vec<u8>),
}

#[derive(Default)]
pub struct Steps<W: World> {
    steps: StepsCollection<W>,
}

impl<W: World> Steps<W> {
    fn resolve_test<'a>(&'a self, step: &Step) -> Option<TestPayload<'a, W>> {
        self.steps.resolve(step)
    }

    fn run_test(
        &self,
        world: &mut W,
        test: TestPayload<'_, W>,
        step: &Step,
        suppress_output: bool,
    ) -> TestResult {
        let test_result = PanicTrap::run(suppress_output, || match test.function {
            TestFunction::Sync(SyncTestFunction::WithArgs(function)) => {
                function(world, &test.payload, step)
            }
            TestFunction::Sync(SyncTestFunction::WithoutArgs(function)) => function(world, step),
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
                        use std::io::{stderr, Write};
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

            let test_type = match self.resolve_test(&step) {
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
        options: &crate::cli::CliOptions,
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
        feature_files: Vec<std::path::PathBuf>,
        before_fns: &[HelperFn],
        after_fns: &[HelperFn],
        options: crate::cli::CliOptions,
        output: &mut impl OutputVisitor,
    ) -> bool {
        use std::convert::TryFrom;

        output.visit_start();

        let mut is_success = true;

        for path in feature_files {
            let feature = match gherkin::Feature::try_from(&*path) {
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
