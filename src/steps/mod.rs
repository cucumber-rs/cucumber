// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

mod builder;
mod collection;

use futures::future::{BoxFuture, FutureExt};
use futures::task::{Context, Poll};
use pin_utils::unsafe_pinned;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::{Arc, RwLock};

pub use self::builder::StepsBuilder;
pub(crate) use self::collection::StepsCollection;
use crate::panic_trap::{PanicDetails, PanicTrap};
use crate::{HelperFn, OutputVisitor, Step, World};

pub struct TestFuture {
    future: BoxFuture<'static, ()>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
impl TestFuture {
    unsafe_pinned!(future: BoxFuture<'static, ()>);

    pub fn new(f: impl Future<Output = ()> + Send + 'static) -> Self {
        TestFuture { future: f.boxed() }
    }
}

impl Future for TestFuture {
    type Output = Result<(), Box<dyn std::any::Any + Send>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        catch_unwind(AssertUnwindSafe(|| self.future().poll(cx)))?.map(Ok)
    }
}

type LiteralSyncTestFunction<W> = fn(&mut W, &Step) -> ();
type ArgsSyncTestFunction<W> = fn(&mut W, &[String], &Step) -> ();
type LiteralAsyncTestFunction<W> = fn(Arc<RwLock<W>>, &Step) -> TestFuture;
type ArgsAsyncTestFunction<W> = fn(Arc<RwLock<W>>, &[String], &Step) -> TestFuture;

pub(crate) struct TestPayload<W: World> {
    function: Arc<TestFunction<W>>,
    payload: Vec<String>,
}

enum SyncTestFunction<W> {
    WithArgs(ArgsSyncTestFunction<W>),
    WithoutArgs(LiteralSyncTestFunction<W>),
}

enum AsyncTestFunction<W> {
    WithArgs(ArgsAsyncTestFunction<W>),
    WithoutArgs(LiteralAsyncTestFunction<W>),
}

enum TestFunction<W> {
    Sync(SyncTestFunction<W>),
    Async(AsyncTestFunction<W>),
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
    fn resolve_test<'a>(&'a self, step: &Step) -> Option<TestPayload<W>> {
        self.steps.resolve(step)
    }

    async fn run_test(
        &self,
        world: &Arc<RwLock<W>>,
        test: TestPayload<W>,
        step: &Arc<Step>,
        suppress_output: bool,
    ) -> TestResult {
        let world = Arc::clone(world);
        let step = Arc::clone(step);

        let fut_result = PanicTrap::run(suppress_output, || {
            let fut = match *test.function {
                TestFunction::Sync(SyncTestFunction::WithArgs(function)) => {
                    TestFuture::new(async move {
                        let mut world = world.write().unwrap();
                        let payload = test.payload;
                        function(&mut *world, &*payload, &*step)
                    })
                }
                TestFunction::Sync(SyncTestFunction::WithoutArgs(function)) => {
                    TestFuture::new(async move {
                        let mut world = world.write().unwrap();
                        function(&mut *world, &*step)
                    })
                }
                TestFunction::Async(AsyncTestFunction::WithArgs(generator)) => {
                    generator(world, &test.payload, &*step)
                }
                TestFunction::Async(AsyncTestFunction::WithoutArgs(generator)) => {
                    generator(world, &*step)
                }
            };
            fut
        });

        let future = match fut_result.result {
            Ok(fut) => fut,
            Err(panic_info) => {
                return if panic_info.payload.ends_with("cucumber test skipped") {
                    TestResult::Skipped
                } else {
                    TestResult::Fail(panic_info, fut_result.stdout, fut_result.stderr)
                }
            }
        };

        let _ = future.await;
        TestResult::Pass
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

        let world = Arc::new(RwLock::new({
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
        }));

        let mut is_success = true;
        let mut is_skipping = false;

        let steps = feature
            .background
            .iter()
            .map(|bg| bg.steps.iter())
            .flatten()
            .chain(scenario.steps.iter())
            .cloned()
            .map(Arc::new);

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
                let result = self
                    .run_test(&world, test_type, &step, suppress_output)
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
        options: &crate::cli::CliOptions,
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
            ));
        }

        // Check if all are successful
        futures::future::join_all(futures)
            .await
            .into_iter()
            .all(|x| x)
    }

    pub async fn run(
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
