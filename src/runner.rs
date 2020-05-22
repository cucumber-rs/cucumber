// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::any::Any;
use std::panic;
use std::pin::Pin;
use std::rc::Rc;

use async_stream::stream;
use futures::{Future, Stream, StreamExt};

use crate::collection::StepsCollection;
use crate::event::*;
use crate::{World, TEST_SKIPPED};

pub(crate) type PanicError = Box<(dyn Any + Send + 'static)>;
pub(crate) type TestFuture<W> = Pin<Box<dyn Future<Output = Result<W, PanicError>>>>;

pub type BasicStepFn<W> = Rc<dyn Fn(W, Rc<gherkin::Step>) -> TestFuture<W>>;
pub type RegexStepFn<W> = Rc<dyn Fn(Vec<String>, W, Rc<gherkin::Step>) -> TestFuture<W>>;

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

pub(crate) struct Runner<W: World> {
    functions: StepsCollection<W>,
    features: Rc<Vec<gherkin::Feature>>,
}

impl<W: World> Runner<W> {
    #[inline]
    pub fn new(
        functions: StepsCollection<W>,
        features: Rc<Vec<gherkin::Feature>>,
    ) -> Rc<Runner<W>> {
        Rc::new(Runner {
            functions,
            features,
        })
    }

    async fn run_step(self: Rc<Self>, step: Rc<gherkin::Step>, world: W) -> TestEvent<W> {
        use std::io::prelude::*;

        let func = match self.functions.resolve(&step) {
            Some(v) => v,
            None => return TestEvent::Unimplemented,
        };

        let mut stdout = shh::stdout().unwrap();
        let mut stderr = shh::stderr().unwrap();

        // This ugly mess here catches the panics from async calls.
        let panic_info = std::sync::Arc::new(std::sync::Mutex::new(None));
        let panic_info0 = std::sync::Arc::clone(&panic_info);
        panic::set_hook(Box::new(move |pi| {
            *panic_info0.lock().unwrap() = Some(PanicInfo {
                location: pi
                    .location()
                    .map(|l| Location {
                        file: l.file().to_string(),
                        line: l.line(),
                        column: l.column(),
                    })
                    .unwrap_or_else(|| Location::unknown()),
                payload: coerce_error(pi.payload()),
            });
        }));

        let result = match func {
            TestFunction::Basic(f) => (f)(world, step).await,
            TestFunction::Regex(f, r) => (f)(r, world, step).await,
        };

        let mut out = String::new();
        let mut err = String::new();
        stdout.read_to_string(&mut out).unwrap_or_else(|_| {
            out = "Error retrieving stdout".to_string();
            0
        });
        stderr.read_to_string(&mut err).unwrap_or_else(|_| {
            err = "Error retrieving stderr".to_string();
            0
        });

        drop(stdout);
        drop(stderr);

        let output = CapturedOutput { out, err };
        match result {
            Ok(w) => TestEvent::Success(w, output),
            Err(e) => {
                let e = coerce_error(&e);
                if &*e == TEST_SKIPPED {
                    return TestEvent::Skipped;
                }

                let mut guard = panic_info.lock().unwrap();
                let pi = guard.take().unwrap_or_else(|| PanicInfo::unknown());
                TestEvent::Failure(pi, output)
            }
        }
    }

    fn run_feature(self: Rc<Self>, feature: Rc<gherkin::Feature>) -> FeatureStream {
        Box::pin(stream! {
            yield FeatureEvent::Starting;

            for scenario in feature.scenarios.iter() {
                let this = Rc::clone(&self);
                let scenario = Rc::new(scenario.clone());

                let mut stream = this.run_scenario(Rc::clone(&scenario), Rc::clone(&feature));

                while let Some(event) = stream.next().await {
                    yield FeatureEvent::Scenario(Rc::clone(&scenario), event);
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

                let mut stream = this.run_scenario(Rc::clone(&scenario), Rc::clone(&feature));

                while let Some(event) = stream.next().await {
                    match event {
                        ScenarioEvent::Failed => { return_event = Some(RuleEvent::Failed); },
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
    ) -> ScenarioStream {
        Box::pin(stream! {
            yield ScenarioEvent::Starting;
            let mut world = Some(W::new().await);

            if let Some(steps) = feature.background.as_ref().map(|x| &x.steps) {
                for step in scenario.steps.iter() {
                    let this = Rc::clone(&self);

                    let step = Rc::new(step.clone());
                    let result = this.run_step(Rc::clone(&step), world.take().unwrap()).await;

                    match result {
                        TestEvent::Success(w, output) => {
                            yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Passed(output));
                            // Pass world result for current step to next step.
                            world = Some(w);
                        }
                        TestEvent::Failure(e, output) => {
                            yield ScenarioEvent::Background(Rc::clone(&step), StepEvent::Failed(output, e));
                            yield ScenarioEvent::Failed;
                            return;
                        },
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

                let step = Rc::new(step.clone());
                let result = this.run_step(Rc::clone(&step), world.take().unwrap()).await;

                match result {
                    TestEvent::Success(w, output) => {
                        yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Passed(output));
                        // Pass world result for current step to next step.
                        world = Some(w);
                    }
                    TestEvent::Failure(e, output) => {
                        yield ScenarioEvent::Step(Rc::clone(&step), StepEvent::Failed(output, e));
                        yield ScenarioEvent::Failed;
                        return;
                    },
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
            yield CucumberEvent::Starting;

            let features = self.features.iter().cloned().map(Rc::new).collect::<Vec<_>>();

            for feature in features.into_iter() {
                let this = Rc::clone(&self);
                let mut stream = this.run_feature(Rc::clone(&feature));

                while let Some(event) = stream.next().await {
                    yield CucumberEvent::Feature(Rc::clone(&feature), event);
                }
            }

            yield CucumberEvent::Finished;
        })
    }
}

type CucumberStream = Pin<Box<dyn Stream<Item = CucumberEvent>>>;
type FeatureStream = Pin<Box<dyn Stream<Item = FeatureEvent>>>;
type RuleStream = Pin<Box<dyn Stream<Item = RuleEvent>>>;
type ScenarioStream = Pin<Box<dyn Stream<Item = ScenarioEvent>>>;
