// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::any::Any;
use std::panic::{self, UnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, TryLockError};

use async_stream::stream;
use futures::{Future, Stream, StreamExt};
use regex::Regex;

use crate::collection::StepsCollection;
use crate::event::*;
use crate::{TestError, World, TEST_SKIPPED};

use super::ExampleValues;
use std::time::{Duration, Instant};

pub(crate) type TestFuture<W> = Pin<Box<dyn Future<Output = Result<W, TestError>>>>;

pub type BasicStepFn<W> = Rc<dyn Fn(W, Rc<gherkin::Step>) -> TestFuture<W> + UnwindSafe>;
pub type RegexStepFn<W> =
    Rc<dyn Fn(W, Vec<String>, Rc<gherkin::Step>) -> TestFuture<W> + UnwindSafe>;

pub enum TestFunction<W> {
    Basic(BasicStepFn<W>),
    Regex(RegexStepFn<W>, Vec<String>),
}

fn coerce_error(err: &(dyn Any + Send + 'static)) -> String {
    if let Some(string) = err.downcast_ref::<String>() {
        string.to_string()
    } else if let Some(string) = err.downcast_ref::<&str>() {
        (*string).to_string()
    } else {
        "(Could not resolve panic payload)".into()
    }
}

/// Stats for various event results
#[derive(Debug, Default, Clone)]
pub struct Stats {
    /// total events seen
    pub total: u32,
    /// events skipped
    pub skipped: u32,
    /// events that passed
    pub passed: u32,
    /// events that failed
    pub failed: u32,
    /// events that timed out
    pub timed_out: u32,
}

impl Stats {
    /// Indicates this has failing states (aka failed or timed_out)
    pub fn failed(&self) -> bool {
        self.failed > 0 || self.timed_out > 0
    }
}

/// The result of the Cucumber run
#[derive(Debug, Clone)]
pub struct RunResult {
    /// the time when the run was started
    pub started: std::time::Instant,
    /// the time the run took
    pub elapsed: std::time::Duration,
    /// Stats of features of this run
    pub features: Stats,
    /// Stats of rules of this run
    pub rules: Stats,
    /// Stats of scenarios of this run
    pub scenarios: Stats,
    /// Stats of scenarios of this run
    pub steps: Stats,
}

impl RunResult {
    /// Indicates this has failing states (aka failed or timed_out)
    pub fn failed(&self) -> bool {
        self.features.failed() || self.scenarios.failed()
    }
}

#[derive(Debug, Clone)]
struct StatsCollector {
    started: std::time::Instant,
    features: Stats,
    rules: Stats,
    scenarios: Stats,
    steps: Stats,
}

impl StatsCollector {
    fn new() -> Self {
        StatsCollector {
            started: std::time::Instant::now(),
            features: Default::default(),
            rules: Default::default(),
            scenarios: Default::default(),
            steps: Default::default(),
        }
    }

    fn handle_rule_event(&mut self, event: &RuleEvent) {
        match event {
            RuleEvent::Starting => {
                self.rules.total += 1;
            }
            RuleEvent::Scenario(_, ref event) => self.handle_scenario_event(event),
            RuleEvent::Skipped => {
                self.rules.skipped += 1;
            }
            RuleEvent::Passed => {
                self.rules.passed += 1;
            }
            RuleEvent::Failed(FailureKind::Panic) => {
                self.rules.failed += 1;
            }
            RuleEvent::Failed(FailureKind::TimedOut) => {
                self.rules.timed_out += 1;
            }
        }
    }

    fn handle_scenario_event(&mut self, event: &ScenarioEvent) {
        match event {
            ScenarioEvent::Starting(_) => {
                self.scenarios.total += 1;
            }
            ScenarioEvent::Background(_, ref event) => self.handle_step_event(event),
            ScenarioEvent::Step(_, ref event) => self.handle_step_event(event),
            ScenarioEvent::Skipped => {
                self.scenarios.skipped += 1;
            }
            ScenarioEvent::Passed => {
                self.scenarios.passed += 1;
            }
            ScenarioEvent::Failed(FailureKind::Panic) => {
                self.scenarios.failed += 1;
            }
            ScenarioEvent::Failed(FailureKind::TimedOut) => {
                self.scenarios.timed_out += 1;
            }
        }
    }

    fn handle_step_event(&mut self, event: &StepEvent) {
        self.steps.total += 1;
        match event {
            StepEvent::Starting => {
                // we don't have to count this
            }
            StepEvent::Unimplemented => {
                self.steps.skipped += 1;
            }
            StepEvent::Skipped => {
                self.steps.skipped += 1;
            }
            StepEvent::Passed(_) => {
                self.steps.passed += 1;
            }
            StepEvent::Failed(StepFailureKind::Panic(_, _)) => {
                self.steps.failed += 1;
            }
            StepEvent::Failed(StepFailureKind::TimedOut) => {
                self.steps.timed_out += 1;
            }
        }
    }

    fn handle_feature_event(&mut self, event: &FeatureEvent) {
        match event {
            FeatureEvent::Starting => {
                self.features.total += 1;
            }
            FeatureEvent::Scenario(_, ref event) => self.handle_scenario_event(event),
            FeatureEvent::Rule(_, ref event) => self.handle_rule_event(event),
            _ => {}
        }
    }

    fn collect(self) -> RunResult {
        let StatsCollector {
            started,
            features,
            rules,
            scenarios,
            steps,
        } = self;

        RunResult {
            elapsed: started.elapsed(),
            started,
            features,
            rules,
            scenarios,
            steps,
        }
    }
}

pub(crate) struct Runner<W: World> {
    functions: StepsCollection<W>,
    features: Rc<Vec<gherkin::Feature>>,
    step_timeout: Option<Duration>,
    enable_capture: bool,
    scenario_filter: Option<Regex>,
}

impl<W: World> Runner<W> {
    #[inline]
    pub fn new(
        functions: StepsCollection<W>,
        features: Rc<Vec<gherkin::Feature>>,
        step_timeout: Option<Duration>,
        enable_capture: bool,
        scenario_filter: Option<Regex>,
    ) -> Rc<Runner<W>> {
        Rc::new(Runner {
            functions,
            features,
            step_timeout,
            enable_capture,
            scenario_filter,
        })
    }

    async fn run_step(self: Rc<Self>, step: Rc<gherkin::Step>, world: W) -> TestEvent<W> {
        use std::io::prelude::*;

        let func = match self.functions.resolve(&step) {
            Some(v) => v,
            None => return TestEvent::Unimplemented,
        };

        let mut maybe_capture_handles = if self.enable_capture {
            Some((shh::stdout().unwrap(), shh::stderr().unwrap()))
        } else {
            None
        };

        // This ugly mess here catches the panics from async calls.
        let panic_info = Arc::new(std::sync::Mutex::new(None));
        let panic_info0 = Arc::clone(&panic_info);
        let step_timeout0 = self.step_timeout;
        panic::set_hook(Box::new(move |pi| {
            let panic_info = Some(PanicInfo {
                location: pi
                    .location()
                    .map(|l| Location {
                        file: l.file().to_string(),
                        line: l.line(),
                        column: l.column(),
                    })
                    .unwrap_or_else(Location::unknown),
                payload: coerce_error(pi.payload()),
            });
            if let Some(step_timeout) = step_timeout0 {
                let start_point = Instant::now();
                loop {
                    match panic_info0.try_lock() {
                        Ok(mut guard) => {
                            *guard = panic_info;
                            return;
                        }
                        Err(TryLockError::WouldBlock) => {
                            if start_point.elapsed() < step_timeout {
                                continue;
                            } else {
                                return;
                            }
                        }
                        Err(TryLockError::Poisoned(_)) => {
                            return;
                        }
                    }
                }
            } else {
                *panic_info0.lock().unwrap() = panic_info;
            }
        }));

        let step_future = match func {
            TestFunction::Basic(f) => (f)(world, step),
            TestFunction::Regex(f, r) => (f)(world, r, step),
        };
        let result = if let Some(step_timeout) = self.step_timeout {
            let timeout = Box::pin(async {
                futures_timer::Delay::new(step_timeout).await;
                Err(TestError::TimedOut)
            });
            futures::future::select(timeout, step_future)
                .await
                .factor_first()
                .0
        } else {
            step_future.await
        };

        let mut out = String::new();
        let mut err = String::new();
        // Note the use of `take` to move the handles into this branch so that they are
        // appropriately dropped following
        if let Some((mut stdout, mut stderr)) = maybe_capture_handles.take() {
            stdout.read_to_string(&mut out).unwrap_or_else(|_| {
                out = "Error retrieving stdout".to_string();
                0
            });
            stderr.read_to_string(&mut err).unwrap_or_else(|_| {
                err = "Error retrieving stderr".to_string();
                0
            });
        }

        let output = CapturedOutput { out, err };
        match result {
            Ok(w) => TestEvent::Success(w, output),
            Err(TestError::TimedOut) => TestEvent::Failure(StepFailureKind::TimedOut),
            Err(TestError::PanicError(e)) => {
                let e = coerce_error(&e);
                if &*e == TEST_SKIPPED {
                    return TestEvent::Skipped;
                }

                let pi = if let Some(step_timeout) = self.step_timeout {
                    let start_point = Instant::now();
                    loop {
                        match panic_info.try_lock() {
                            Ok(mut guard) => {
                                break guard.take().unwrap_or_else(PanicInfo::unknown);
                            }
                            Err(TryLockError::WouldBlock) => {
                                if start_point.elapsed() < step_timeout {
                                    futures_timer::Delay::new(Duration::from_micros(10)).await;
                                    continue;
                                } else {
                                    break PanicInfo::unknown();
                                }
                            }
                            Err(TryLockError::Poisoned(_)) => break PanicInfo::unknown(),
                        }
                    }
                } else {
                    let mut guard = panic_info.lock().unwrap();
                    guard.take().unwrap_or_else(PanicInfo::unknown)
                };
                TestEvent::Failure(StepFailureKind::Panic(output, pi))
            }
        }
    }

    fn run_feature(self: Rc<Self>, feature: Rc<gherkin::Feature>) -> FeatureStream {
        Box::pin(stream! {
            yield FeatureEvent::Starting;

            for scenario in feature.scenarios.iter() {
                // If regex filter fails, skip the scenario
                if let Some(ref regex) = self.scenario_filter {
                    if !regex.is_match(&scenario.name) {
                        continue;
                    }
                }

                let examples = ExampleValues::from_examples(&scenario.examples);
                for example_values in examples {
                    let this = Rc::clone(&self);
                    let scenario = Rc::new(scenario.clone());

                    let mut stream = this.run_scenario(Rc::clone(&scenario), Rc::clone(&feature), example_values);

                    while let Some(event) = stream.next().await {
                        yield FeatureEvent::Scenario(Rc::clone(&scenario), event);

                    }
                }
            }

            for rule in feature.rules.iter() {
                let this = Rc::clone(&self);
                let rule = Rc::new(rule.clone());

                let mut stream = this.run_rule(Rc::clone(&rule), Rc::clone(&feature));

                while let Some(event) = stream.next().await {
                    yield FeatureEvent::Rule(Rc::clone(&rule), event);
                }
            }

            yield FeatureEvent::Finished;
        })
    }

    fn run_rule(
        self: Rc<Self>,
        rule: Rc<gherkin::Rule>,
        feature: Rc<gherkin::Feature>,
    ) -> RuleStream {
        Box::pin(stream! {
            yield RuleEvent::Starting;

            let mut return_event = None;

            for scenario in rule.scenarios.iter() {
                let this = Rc::clone(&self);
                let scenario = Rc::new(scenario.clone());

                let mut stream = this.run_scenario(Rc::clone(&scenario), Rc::clone(&feature), ExampleValues::empty());

                while let Some(event) = stream.next().await {
                    match event {
                        ScenarioEvent::Failed(FailureKind::Panic) => { return_event = Some(RuleEvent::Failed(FailureKind::Panic)); },
                        ScenarioEvent::Failed(FailureKind::TimedOut) => { return_event = Some(RuleEvent::Failed(FailureKind::TimedOut)); },
                        ScenarioEvent::Passed if return_event.is_none() => { return_event = Some(RuleEvent::Passed); },
                        ScenarioEvent::Skipped if return_event == Some(RuleEvent::Passed) => { return_event = Some(RuleEvent::Skipped); }
                        _ => {}
                    }
                    yield RuleEvent::Scenario(Rc::clone(&scenario), event);
                }
            }

            yield return_event.unwrap_or(RuleEvent::Skipped);
        })
    }

    fn run_scenario(
        self: Rc<Self>,
        scenario: Rc<gherkin::Scenario>,
        feature: Rc<gherkin::Feature>,
        example: super::ExampleValues,
    ) -> ScenarioStream {
        Box::pin(stream! {
            yield ScenarioEvent::Starting(example.clone());
            let mut world = Some(W::new().await.unwrap());

            if let Some(steps) = feature.background.as_ref().map(|x| &x.steps) {
                for step in steps.iter() {
                    let this = Rc::clone(&self);
                    let step = Rc::new(step.clone());

                    yield ScenarioEvent::Background(Rc::clone(&step), StepEvent::Starting);

                    let result = this.run_step(Rc::clone(&step), world.take().unwrap()).await;

                    match result {
                        TestEvent::Success(w, output) => {
                            yield ScenarioEvent::Background(Rc::clone(&step), StepEvent::Passed(output));
                            // Pass world result for current step to next step.
                            world = Some(w);
                        }
                        TestEvent::Failure(StepFailureKind::Panic(output, e)) => {
                            yield ScenarioEvent::Background(Rc::clone(&step), StepEvent::Failed(StepFailureKind::Panic(output, e)));
                            yield ScenarioEvent::Failed(FailureKind::Panic);
                            return;
                        },
                        TestEvent::Failure(StepFailureKind::TimedOut) => {
                            yield ScenarioEvent::Background(Rc::clone(&step), StepEvent::Failed(StepFailureKind::TimedOut));
                            yield ScenarioEvent::Failed(FailureKind::TimedOut);
                            return;
                        }
                        TestEvent::Skipped => {
                            yield ScenarioEvent::Background(Rc::clone(&step), StepEvent::Skipped);
                            yield ScenarioEvent::Skipped;
                            return;
                        }
                        TestEvent::Unimplemented => {
                            yield ScenarioEvent::Background(Rc::clone(&step), StepEvent::Unimplemented);
                            yield ScenarioEvent::Skipped;
                            return;
                        }
                    }
                }
            }

            for step in scenario.steps.iter() {
                let this = Rc::clone(&self);

                let mut step = step.clone();
                if !example.is_empty() {
                    step.value = example.insert_values(&step.value);
                }
                let step = Rc::new(step);

                yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Starting);

                let result = this.run_step(Rc::clone(&step), world.take().unwrap()).await;

                match result {
                    TestEvent::Success(w, output) => {
                        yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Passed(output));
                        // Pass world result for current step to next step.
                        world = Some(w);
                    }
                    TestEvent::Failure(StepFailureKind::Panic(output, e)) => {
                        yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Failed(StepFailureKind::Panic(output, e)));
                        yield ScenarioEvent::Failed(FailureKind::Panic);
                        return;
                    },
                    TestEvent::Failure(StepFailureKind::TimedOut) => {
                        yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Failed(StepFailureKind::TimedOut));
                        yield ScenarioEvent::Failed(FailureKind::TimedOut);
                        return;
                    }
                    TestEvent::Skipped => {
                        yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Skipped);
                        yield ScenarioEvent::Skipped;
                        return;
                    }
                    TestEvent::Unimplemented => {
                        yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Unimplemented);
                        yield ScenarioEvent::Skipped;
                        return;
                    }
                }
            }

            yield ScenarioEvent::Passed;
        })
    }

    pub fn run(self: Rc<Self>) -> CucumberStream {
        Box::pin(stream! {
            let mut stats = StatsCollector::new();
            yield CucumberEvent::Starting;

            let features = self.features.iter().cloned().map(Rc::new).collect::<Vec<_>>();
            for feature in features.into_iter() {
                let this = Rc::clone(&self);
                let mut stream = this.run_feature(Rc::clone(&feature));

                while let Some(event) = stream.next().await {
                    stats.handle_feature_event(&event);
                    yield CucumberEvent::Feature(Rc::clone(&feature), event);
                }
            }

            yield CucumberEvent::Finished(stats.collect());
        })
    }
}

type CucumberStream = Pin<Box<dyn Stream<Item = CucumberEvent>>>;
type FeatureStream = Pin<Box<dyn Stream<Item = FeatureEvent>>>;
type RuleStream = Pin<Box<dyn Stream<Item = RuleEvent>>>;
type ScenarioStream = Pin<Box<dyn Stream<Item = ScenarioEvent>>>;
