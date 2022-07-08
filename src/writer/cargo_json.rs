// Copyright (c) 2018-2022  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Cucumber JSON format][1] [`Writer`] implementation.
//!
//! [1]: https://github.com/cucumber/cucumber-json-schema

use std::{
    fmt::Debug,
    io, iter, mem,
    sync::Arc,
    time::{Duration, SystemTime},
};

use async_trait::async_trait;
use itertools::Itertools as _;
use serde::Serialize;

use crate::{
    event, parser,
    writer::{self, basic::coerce_error, out::WriteStrExt as _},
    Event, World, Writer,
};

/// TODO
#[derive(Debug, Clone, clap::Args)]
pub struct Cli {
    /// TODO
    #[clap(long)]
    pub format: Option<String>,

    /// TODO
    #[clap(long)]
    pub show_output: bool,

    /// TODO
    #[clap(short = 'Z')]
    pub unstable: Option<String>,
}

/// [Cucumber JSON format][1] [`Writer`] implementation outputting JSON to an
/// [`io::Write`] implementor.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into
/// a [`writer::Normalize`], otherwise will panic in runtime as won't be able to
/// form [correct JSON][1].
///
/// [1]: https://github.com/cucumber/cucumber-json-schema
/// [`Normalized`]: writer::Normalized
#[derive(Clone, Debug)]
pub struct CargoJson<W, Out: io::Write = io::Stdout> {
    /// [`io::Write`] implementor to output [JSON][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    output: Out,

    /// Collection of [`Feature`]s to output [JSON][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    features: Vec<parser::Result<Event<event::Cucumber<W>>>>,

    /// TODO
    parsed_all: bool,

    /// TODO
    passed: usize,

    /// TODO
    failed: usize,

    parsing_failed: usize,

    /// TODO
    ignored: usize,

    /// [`SystemTime`] when the current [`Hook`]/[`Step`] has started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Hook`]: event::Hook
    started: Option<SystemTime>,
}

#[async_trait(?Send)]
impl<W: World + Debug, Out: io::Write> Writer<W> for CargoJson<W, Out> {
    type Cli = Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        use event::Cucumber;

        let unite = |ev: Result<(Cucumber<W>, event::Metadata), _>| {
            ev.map(|(e, m)| m.insert(e))
        };

        match (event.map(Event::split), self.parsed_all) {
            (ev @ Ok((Cucumber::ParsingFinished { .. }, _)), false) => {
                self.parsed_all = true;
                let all_events =
                    iter::once(unite(ev)).chain(mem::take(&mut self.features));
                for ev in all_events {
                    self.output_event(ev);
                }
            }
            (ev, false) => self.features.push(unite(ev)),
            (ev, true) => self.output_event(unite(ev)),
        }
    }
}

impl<W: Debug, Out: io::Write> CargoJson<W, Out> {
    /// TODO
    fn output_event(&mut self, ev: parser::Result<Event<event::Cucumber<W>>>) {
        for ev in self.into_lib_test_event(ev) {
            self.output
                .write_line(serde_json::to_string(&ev).unwrap())
                .unwrap();
        }
    }

    /// TODO
    fn into_lib_test_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
    ) -> Vec<LibTestJsonEvent> {
        use event::Cucumber;

        match ev.map(Event::split) {
            Ok((Cucumber::Started, meta)) => {
                self.started = Some(meta.at);
                Vec::new()
            }
            Ok((Cucumber::ParsingFinished { steps, .. }, _)) => {
                vec![LibTestJsonEvent::Suite {
                    event: SuiteEvent::Started { test_count: steps },
                }]
            }
            Ok((Cucumber::Finished, _)) => {
                let exec_time = self
                    .started
                    .and_then(|started| {
                        SystemTime::now().duration_since(started).ok()
                    })
                    .as_ref()
                    .map(Duration::as_secs_f64);

                let failed = self.failed + self.parsing_failed;
                let results = SuiteResults {
                    passed: self.passed,
                    failed,
                    ignored: self.ignored,
                    measured: 0,
                    filtered_out: 0,
                    exec_time,
                };
                vec![if failed == 0 {
                    LibTestJsonEvent::Suite {
                        event: SuiteEvent::Ok { results },
                    }
                } else {
                    LibTestJsonEvent::Suite {
                        event: SuiteEvent::Failed { results },
                    }
                }]
            }
            Ok((Cucumber::Feature(feature, ev), _)) => {
                self.from_feature_event(&feature, ev)
            }
            Err(e) => {
                self.parsing_failed += 1;

                let path = match &e {
                    parser::Error::Parsing(e) => match &**e {
                        gherkin::ParseFileError::Parsing { path, .. }
                        | gherkin::ParseFileError::Reading { path, .. } => {
                            Some(path)
                        }
                    },
                    parser::Error::ExampleExpansion(e) => e.path.as_ref(),
                };
                let name = path.and_then(|p| p.to_str()).map_or_else(
                    || self.parsing_failed.to_string(),
                    |p| p.escape_default().to_string(),
                );
                let name = format!("Parsing {name}");

                vec![
                    LibTestJsonEvent::Test {
                        event: TestEvent::Started { name: name.clone() },
                    },
                    LibTestJsonEvent::Test {
                        event: TestEvent::Failed {
                            name,
                            stdout: Some(e.to_string()),
                            stderr: None,
                        },
                    },
                ]
            }
        }
    }

    /// TODO
    fn from_feature_event(
        &mut self,
        feature: &gherkin::Feature,
        ev: event::Feature<W>,
    ) -> Vec<LibTestJsonEvent> {
        use event::{Feature, Hook, Rule, Scenario, Step};

        let mut from_scenario = |rule: Option<Arc<gherkin::Rule>>,
                                 scenario: Arc<gherkin::Scenario>,
                                 ev| {
            let name = |step: Option<String>| {
                let feature = format!(
                    "{} {}",
                    feature.name,
                    feature
                        .path
                        .as_ref()
                        .and_then(|p| p.to_str())
                        .map(|s| s.escape_default().to_string())
                        .unwrap_or_default(),
                );
                let rule = rule.as_ref().map(|r| {
                    format!("{}: {} {}", r.position.line, r.keyword, r.name)
                });
                let scenario = format!(
                    "{}: {} {}",
                    scenario.position.line, scenario.keyword, scenario.name,
                );
                let name = [
                    Some(&feature),
                    rule.as_ref(),
                    Some(&scenario),
                    step.as_ref(),
                ]
                .into_iter()
                .flatten()
                .join("::");
                name
            };
            let mut from_step = |step: Arc<gherkin::Step>, ev, is_bg: bool| {
                let step = format!(
                    "{}: {}{}{}",
                    step.position.line,
                    is_bg.then_some("Background ").unwrap_or_default(),
                    step.keyword,
                    step.value,
                );
                let name = name(Some(step));

                vec![match ev {
                    Step::Started => LibTestJsonEvent::Test {
                        event: TestEvent::Started { name },
                    },
                    Step::Passed(_) => {
                        self.passed += 1;
                        LibTestJsonEvent::Test {
                            event: TestEvent::Ok { name },
                        }
                    }
                    Step::Skipped => {
                        self.ignored += 1;
                        LibTestJsonEvent::Test {
                            event: TestEvent::Ignored { name },
                        }
                    }
                    Step::Failed(_, world, err) => {
                        self.failed += 1;
                        LibTestJsonEvent::Test {
                            event: TestEvent::Failed {
                                name,
                                stdout: Some(format!(
                                    "{}{}",
                                    err.to_string(),
                                    world
                                        .map(|w| format!("\n{:#?}", w))
                                        .unwrap_or_default(),
                                )),
                                stderr: None,
                            },
                        }
                    }
                }]
            };

            match ev {
                Scenario::Started
                | Scenario::Finished
                | Scenario::Hook(_, Hook::Started | Hook::Passed) => Vec::new(),
                Scenario::Hook(ty, Hook::Failed(w, info)) => {
                    let hook = format!("{} hook", ty);
                    let name = name(Some(hook));
                    vec![
                        LibTestJsonEvent::Test {
                            event: TestEvent::Started { name: name.clone() },
                        },
                        LibTestJsonEvent::Test {
                            event: TestEvent::Failed {
                                name,
                                stdout: Some(format!(
                                    "{}{}",
                                    coerce_error(&info),
                                    w.map(|w| format!("\n{:#?}", w))
                                        .unwrap_or_default(),
                                )),
                                stderr: None,
                            },
                        },
                    ]
                }
                Scenario::Step(step, ev) => from_step(step, ev, false),
                Scenario::Background(step, ev) => from_step(step, ev, true),
            }
        };

        match ev {
            Feature::Started
            | Feature::Finished
            | Feature::Rule(_, Rule::Started | Rule::Finished) => Vec::new(),
            Feature::Rule(rule, Rule::Scenario(scenario, ev)) => {
                from_scenario(Some(rule), scenario, ev)
            }
            Feature::Scenario(scenario, ev) => {
                from_scenario(None, scenario, ev)
            }
        }
    }
}

/// TODO
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LibTestJsonEvent {
    /// TODO
    Suite {
        /// TODO
        #[serde(flatten)]
        event: SuiteEvent,
    },

    /// TODO
    Test {
        /// TODO
        #[serde(flatten)]
        event: TestEvent,
    },
}

/// TODO
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum SuiteEvent {
    /// TODO
    Started {
        /// TODO
        test_count: usize,
    },

    /// TODO
    Ok {
        /// TODO
        #[serde(flatten)]
        results: SuiteResults,
    },

    /// TODO
    Failed {
        /// TODO
        #[serde(flatten)]
        results: SuiteResults,
    },
}

/// TODO
#[derive(Debug, Serialize)]
struct SuiteResults {
    /// TODO
    passed: usize,

    /// TODO
    failed: usize,

    /// TODO
    ignored: usize,

    /// TODO
    measured: usize,

    /// TODO
    filtered_out: usize,

    /// TODO
    #[serde(skip_serializing_if = "Option::is_none")]
    exec_time: Option<f64>,
}

/// TODO
#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum TestEvent {
    /// TODO
    Started {
        /// TODO
        name: String,
    },

    /// TODO
    Ok {
        /// TODO
        name: String,
    },

    /// TODO
    Failed {
        /// TODO
        name: String,

        /// TODO
        #[serde(skip_serializing_if = "Option::is_none")]
        stdout: Option<String>,

        /// TODO
        #[serde(skip_serializing_if = "Option::is_none")]
        stderr: Option<String>,
    },

    /// TODO
    Ignored {
        /// TODO
        name: String,
    },

    /// TODO
    #[allow(dead_code)]
    Timeout {
        /// TODO
        name: String,
    },
}

// ------------

impl<W: World, O: io::Write> writer::NonTransforming for CargoJson<W, O> {}

impl<W: World, O: io::Write> writer::Normalized for CargoJson<W, O> {}

impl<W: World + Debug, O: io::Write> writer::Failure<W> for CargoJson<W, O> {
    fn failed_steps(&self) -> usize {
        self.failed
    }

    fn parsing_errors(&self) -> usize {
        self.parsing_failed
    }

    fn hook_errors(&self) -> usize {
        // TODO
        0
    }
}

impl<W: Debug + World> CargoJson<W, io::Stdout> {
    /// TODO
    pub fn stdout() -> Self {
        Self::new(io::stdout())
    }
}

impl<W: Debug + World, Out: io::Write> CargoJson<W, Out> {
    /// Creates a new [`Normalized`] [`Json`] [`Writer`] outputting [JSON][1]
    /// into the given `output`.
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    #[must_use]
    pub const fn new(output: Out) -> Self {
        Self {
            output,
            features: Vec::new(),
            parsed_all: false,
            passed: 0,
            failed: 0,
            parsing_failed: 0,
            ignored: 0,
            started: None,
        }
    }

    // /// Creates a new non-[`Normalized`] [`Json`] [`Writer`] outputting
    // /// [JSON][1] into the given `output`, and suitable for feeding into
    // /// [`tee()`].
    // ///
    // /// [`Normalized`]: writer::Normalized
    // /// [`tee()`]: crate::WriterExt::tee
    // /// [1]: https://github.com/cucumber/cucumber-json-schema
    // /// [2]: crate::event::Cucumber
    // #[must_use]
    // pub fn for_tee(output: Out) ->
    // discard::Arbitrary<discard::Failure<Self>> {
    //     Self::raw(output)
    //         .discard_failure_writes()
    //         .discard_arbitrary_writes()
    // }

    // /// Handles the given [`event::Scenario`].
    // fn handle_scenario_event<W>(
    //     &mut self,
    //     feature: &gherkin::Feature,
    //     rule: Option<&gherkin::Rule>,
    //     scenario: &gherkin::Scenario,
    //     ev: event::Scenario<W>,
    //     meta: event::Metadata,
    // ) {
    //     use event::Scenario;
    //
    //     match ev {
    //         Scenario::Hook(ty, ev) => {
    //             self
    // .handle_hook_event(feature, rule, scenario, ty, ev, meta);
    //         }
    //         Scenario::Background(st, ev) => {
    //             self.handle_step_event(
    //                 feature,
    //                 rule,
    //                 scenario,
    //                 "background",
    //                 &st,
    //                 ev,
    //                 meta,
    //             );
    //         }
    //         Scenario::Step(st, ev) => {
    //             self.handle_step_event(
    //                 feature, rule, scenario, "scenario", &st, ev, meta,
    //             );
    //         }
    //         Scenario::Started | Scenario::Finished => {}
    //     }
    // }
    //
    // /// Handles the given [`event::Hook`].
    // fn handle_hook_event<W>(
    //     &mut self,
    //     feature: &gherkin::Feature,
    //     rule: Option<&gherkin::Rule>,
    //     scenario: &gherkin::Scenario,
    //     hook_ty: event::HookType,
    //     event: event::Hook<W>,
    //     meta: event::Metadata,
    // ) {
    //     use event::{Hook, HookType};
    //
    //     let mut duration = || {
    //         let started = self.started.take().unwrap_or_else(|| {
    //             panic!("No `Started` event for `{hook_ty} Hook`")
    //         });
    //         meta.at
    //             .duration_since(started)
    //             .unwrap_or_else(|e| {
    //                 panic!(
    //                     "Failed to compute duration between {:?} and \
    //                      {started:?}: {e}",
    //                     meta.at,
    //                 );
    //             })
    //             .as_nanos()
    //     };
    //
    //     let res = match event {
    //         Hook::Started => {
    //             self.started = Some(meta.at);
    //             return;
    //         }
    //         Hook::Passed => HookResult {
    //             result: RunResult {
    //                 status: Status::Passed,
    //                 duration: duration(),
    //                 error_message: None,
    //             },
    //         },
    //         Hook::Failed(_, info) => HookResult {
    //             result: RunResult {
    //                 status: Status::Failed,
    //                 duration: duration(),
    //                 error_message: Some(coerce_error(&info).into_owned()),
    //             },
    //         },
    //     };
    //
    //     let el =
    //         self.mut_or_insert_element(feature, rule, scenario, "scenario");
    //     match hook_ty {
    //         HookType::Before => el.before.push(res),
    //         HookType::After => el.after.push(res),
    //     }
    // }
    //
    // /// Handles the given [`event::Step`].
    // #[allow(clippy::too_many_arguments)]
    // fn handle_step_event<W>(
    //     &mut self,
    //     feature: &gherkin::Feature,
    //     rule: Option<&gherkin::Rule>,
    //     scenario: &gherkin::Scenario,
    //     ty: &'static str,
    //     step: &gherkin::Step,
    //     event: event::Step<W>,
    //     meta: event::Metadata,
    // ) {
    //     let mut duration = || {
    //         let started = self.started.take().unwrap_or_else(|| {
    //             panic!("No `Started` event for `Step` '{}'", step.value)
    //         });
    //         meta.at
    //             .duration_since(started)
    //             .unwrap_or_else(|e| {
    //                 panic!(
    //                     "Failed to compute duration between {:?} and \
    //                      {started:?}: {e}",
    //                     meta.at,
    //                 );
    //             })
    //             .as_nanos()
    //     };
    //
    //     let result = match event {
    //         event::Step::Started => {
    //             self.started = Some(meta.at);
    //             let _ = self
    // .mut_or_insert_element(feature, rule, scenario, ty);
    //             return;
    //         }
    //         event::Step::Passed(..) => RunResult {
    //             status: Status::Passed,
    //             duration: duration(),
    //             error_message: None,
    //         },
    //         event::Step::Failed(_, _, err) => match err {
    //             event::StepError::AmbiguousMatch(err) => RunResult {
    //                 status: Status::Ambiguous,
    //                 duration: duration(),
    //                 error_message: Some(err.to_string()),
    //             },
    //             event::StepError::Panic(info) => RunResult {
    //                 status: Status::Failed,
    //                 duration: duration(),
    //                 error_message: Some(coerce_error(&info).into_owned()),
    //             },
    //         },
    //         event::Step::Skipped => RunResult {
    //             status: Status::Skipped,
    //             duration: duration(),
    //             error_message: None,
    //         },
    //     };
    //
    //     let el = self.mut_or_insert_element(feature, rule, scenario, ty);
    //     el.steps.push(Step {
    //         keyword: step.keyword.clone(),
    //         line: step.position.line,
    //         name: step.value.clone(),
    //         hidden: false,
    //         result,
    //     });
    // }
    //
    // /// Inserts the given `scenario`, if not present, and then returns a
    // /// mutable
    // /// reference to the contained value.
    // fn mut_or_insert_element(
    //     &mut self,
    //     feature: &gherkin::Feature,
    //     rule: Option<&gherkin::Rule>,
    //     scenario: &gherkin::Scenario,
    //     ty: &'static str,
    // ) -> &mut Element {
    //     let f_pos = self
    //         .features
    //         .iter()
    //         .position(|f| f == feature)
    //         .unwrap_or_else(|| {
    //             self.features.push(Feature::new(feature));
    //             self.features.len() - 1
    //         });
    //     let f = self
    //         .features
    //         .get_mut(f_pos)
    //         .unwrap_or_else(|| unreachable!());
    //
    //     let el_pos = f
    //         .elements
    //         .iter()
    //         .position(|el| {
    //             el.name
    //                 == format!(
    //                     "{}{}",
    //                     rule.map(|r| format!("{} ", r.name))
    //                         .unwrap_or_default(),
    //                     scenario.name,
    //                 )
    //                 && el.line == scenario.position.line
    //                 && el.r#type == ty
    //         })
    //         .unwrap_or_else(|| {
    //             f.elements.push(Element::new(feature, rule, scenario, ty));
    //             f.elements.len() - 1
    //         });
    //     f.elements.get_mut(el_pos).unwrap_or_else(|| unreachable!())
    // }
}

// /// [`Serialize`]able tag of a [`gherkin::Feature`] or a
// /// [`gherkin::Scenario`].
// #[derive(Clone, Debug, Serialize)]
// pub struct Tag {
//     /// Name of this [`Tag`].
//     pub name: String,
//
//     /// Line number of this [`Tag`] in a `.feature` file.
//     ///
//     /// As [`gherkin`] parser omits this info, line number is taken from
//     /// [`gherkin::Feature`] or [`gherkin::Scenario`].
//     pub line: usize,
// }
//
// pub use self::status::Status;
//
// /// TODO: Only because of [`Serialize`] deriving, try to remove on next
// ///       [`serde`] update.
// #[allow(clippy::use_self, clippy::wildcard_imports)]
// mod status {
//     use super::*;
//
//     /// Possible statuses of running [`gherkin::Step`].
//     #[derive(Clone, Copy, Debug, Serialize)]
//     pub enum Status {
//         /// [`event::Step::Passed`].
//         Passed,
//
//         /// [`event::Step::Failed`] with an [`event::StepError::Panic`].
//         Failed,
//
//         /// [`event::Step::Skipped`].
//         Skipped,
//
//         /// [`event::Step::Failed`] with an
//         /// [`event::StepError::AmbiguousMatch`].
//         Ambiguous,
//
//         /// Never constructed and is here only to fully describe
//         /// [JSON schema][1].
//         ///
//         /// [1]: https://github.com/cucumber/cucumber-json-schema
//         Undefined,
//
//         /// Never constructed and is here only to fully describe
//         /// [JSON schema][1].
//         ///
//         /// [1]: https://github.com/cucumber/cucumber-json-schema
//         Pending,
//     }
// }
//
// /// [`Serialize`]able result of running something.
// #[derive(Clone, Debug, Serialize)]
// pub struct RunResult {
//     /// [`Status`] of this running result.
//     pub status: Status,
//
//     /// Execution time.
//     ///
//     /// While nowhere being documented, [`cucumber-jvm` uses nanoseconds][1].
//     ///
//     /// [1]: https://tinyurl.com/34wry46u#L325
//     pub duration: u128,
//
//     /// Error message of [`Status::Failed`] or [`Status::Ambiguous`]
//     /// (if any).
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub error_message: Option<String>,
// }
//
// /// [`Serialize`]able [`gherkin::Step`].
// #[derive(Clone, Debug, Serialize)]
// pub struct Step {
//     /// [`gherkin::Step::keyword`].
//     pub keyword: String,
//
//     /// [`gherkin::Step`] line number in a `.feature` file.
//     pub line: usize,
//
//     /// [`gherkin::Step::value`].
//     pub name: String,
//
//     /// Never [`true`] and is here only to fully describe a [JSON schema][1].
//     ///
//     /// [1]: https://github.com/cucumber/cucumber-json-schema
//     #[serde(skip_serializing_if = "std::ops::Not::not")]
//     pub hidden: bool,
//
//     /// [`RunResult`] of this [`Step`].
//     pub result: RunResult,
// }
//
// /// [`Serialize`]able result of running a [`Before`] or [`After`] hook.
// ///
// /// [`Before`]: event::HookType::Before
// /// [`After`]: event::HookType::After
// #[derive(Clone, Debug, Serialize)]
// pub struct HookResult {
//     /// [`RunResult`] of the hook.
//     pub result: RunResult,
// }
//
// /// [`Serialize`]able [`gherkin::Background`] or [`gherkin::Scenario`].
// #[derive(Clone, Debug, Serialize)]
// pub struct Element {
//     /// Doesn't appear in the [JSON schema][1], but present in
//     /// [its generated test cases][2].
//     ///
//     /// [1]: https://github.com/cucumber/cucumber-json-schema
//     /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     pub after: Vec<HookResult>,
//
//     /// Doesn't appear in the [JSON schema][1], but present in
//     /// [its generated test cases][2].
//     ///
//     /// [1]: https://github.com/cucumber/cucumber-json-schema
//     /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
//     #[serde(skip_serializing_if = "Vec::is_empty")]
//     pub before: Vec<HookResult>,
//
//     /// [`gherkin::Scenario::keyword`].
//     pub keyword: String,
//
//     /// Type of this [`Element`].
//     ///
//     /// Only set to `background` or `scenario`, but [JSON schema][1] doesn't
//     /// constraint only to those values, so maybe a subject to change.
//     ///
//     /// [1]: https://github.com/cucumber/cucumber-json-schema
//     pub r#type: &'static str,
//
//     /// Identifier of this [`Element`]. Doesn't have to be unique.
//     pub id: String,
//
//     /// [`gherkin::Scenario`] line number inside a `.feature` file.
//     pub line: usize,
//
//     /// [`gherkin::Scenario::name`], optionally prepended with a
//     /// [`gherkin::Rule::name`].
//     ///
//     /// This is done because [JSON schema][1] doesn't support
//     /// [`gherkin::Rule`]s
//     /// at the moment.
//     ///
//     /// [1]: https://github.com/cucumber/cucumber-json-schema
//     pub name: String,
//
//     /// [`gherkin::Scenario::tags`].
//     pub tags: Vec<Tag>,
//
//     /// [`gherkin::Scenario`]'s [`Step`]s.
//     pub steps: Vec<Step>,
// }
//
// impl Element {
//     /// Creates a new [`Element`] out of the given values.
//     fn new(
//         feature: &gherkin::Feature,
//         rule: Option<&gherkin::Rule>,
//         scenario: &gherkin::Scenario,
//         ty: &'static str,
//     ) -> Self {
//         Self {
//             after: Vec::new(),
//             before: Vec::new(),
//             keyword: (ty == "background")
//                 .then(|| feature.background.as_ref().map(|bg| &bg.keyword))
//                 .flatten()
//                 .unwrap_or(&scenario.keyword)
//                 .clone(),
//             r#type: ty,
//             id: format!(
//                 "{}{}/{}",
//                 feature.name.to_kebab_case(),
//                 rule.map(|r| format!("/{}", r.name.to_kebab_case()))
//                     .unwrap_or_default(),
//                 scenario.name.to_kebab_case(),
//             ),
//             line: scenario.position.line,
//             name: format!(
//                 "{}{}",
//                 rule.map(|r| format!("{} ", r.name)).unwrap_or_default(),
//                 scenario.name.clone(),
//             ),
//             tags: scenario
//                 .tags
//                 .iter()
//                 .map(|t| Tag {
//                     name: t.clone(),
//                     line: scenario.position.line,
//                 })
//                 .collect(),
//             steps: Vec::new(),
//         }
//     }
// }
//
// /// [`Serialize`]able [`gherkin::Feature`].
// #[derive(Clone, Debug, Serialize)]
// pub struct Feature {
//     /// [`gherkin::Feature::path`].
//     pub uri: Option<String>,
//
//     /// [`gherkin::Feature::keyword`].
//     pub keyword: String,
//
//     /// [`gherkin::Feature::name`].
//     pub name: String,
//
//     /// [`gherkin::Feature::tags`].
//     pub tags: Vec<Tag>,
//
//     /// [`gherkin::Feature`]'s [`Element`]s.
//     pub elements: Vec<Element>,
// }
//
// impl Feature {
//     /// Creates a new [`Feature`] out of the given [`gherkin::Feature`].
//     fn new(feature: &gherkin::Feature) -> Self {
//         Self {
//             uri: feature
//                 .path
//                 .as_ref()
//                 .and_then(|p| p.to_str())
//                 .map(str::to_owned),
//             keyword: feature.keyword.clone(),
//             name: feature.name.clone(),
//             tags: feature
//                 .tags
//                 .iter()
//                 .map(|tag| Tag {
//                     name: tag.clone(),
//                     line: feature.position.line,
//                 })
//                 .collect(),
//             elements: Vec::new(),
//         }
//     }
//
//     /// Creates a new [`Feature`] from the given [`ExpandExamplesError`].
//     fn example_expansion_err(err: &ExpandExamplesError) -> Self {
//         Self {
//             uri: err
//                 .path
//                 .as_ref()
//                 .and_then(|p| p.to_str())
//                 .map(str::to_owned),
//             keyword: String::new(),
//             name: String::new(),
//             tags: Vec::new(),
//             elements: vec![Element {
//                 after: Vec::new(),
//                 before: Vec::new(),
//                 keyword: String::new(),
//                 r#type: "scenario",
//                 id: format!(
//                     "failed-to-expand-examples{}",
//                     err.path
//                         .as_ref()
//                         .and_then(|p| p.to_str())
//                         .unwrap_or_default(),
//                 ),
//                 line: 0,
//                 name: String::new(),
//                 tags: Vec::new(),
//                 steps: vec![Step {
//                     keyword: String::new(),
//                     line: err.pos.line,
//                     name: "scenario".into(),
//                     hidden: false,
//                     result: RunResult {
//                         status: Status::Failed,
//                         duration: 0,
//                         error_message: Some(err.to_string()),
//                     },
//                 }],
//             }],
//         }
//     }
//
//     /// Creates a new [`Feature`] from the given [`gherkin::ParseFileError`].
//     fn parsing_err(err: &gherkin::ParseFileError) -> Self {
//         let path = match err {
//             gherkin::ParseFileError::Reading { path, .. }
//             | gherkin::ParseFileError::Parsing { path, .. } => path,
//         }
//         .to_str()
//         .map(str::to_owned);
//
//         Self {
//             uri: path.clone(),
//             keyword: String::new(),
//             name: String::new(),
//             tags: vec![],
//             elements: vec![Element {
//                 after: Vec::new(),
//                 before: Vec::new(),
//                 keyword: String::new(),
//                 r#type: "scenario",
//                 id: format!(
//                     "failed-to-parse{}",
//                     path.as_deref().unwrap_or_default(),
//                 ),
//                 line: 0,
//                 name: String::new(),
//                 tags: Vec::new(),
//                 steps: vec![Step {
//                     keyword: String::new(),
//                     line: 0,
//                     name: "scenario".into(),
//                     hidden: false,
//                     result: RunResult {
//                         status: Status::Failed,
//                         duration: 0,
//                         error_message: Some(err.to_string()),
//                     },
//                 }],
//             }],
//         }
//     }
// }
//
// impl PartialEq<gherkin::Feature> for Feature {
//     fn eq(&self, feature: &gherkin::Feature) -> bool {
//         self.uri
//             .as_ref()
//             .and_then(|uri| {
//                 feature
//                     .path
//                     .as_ref()
//                     .and_then(|p| p.to_str())
//                     .map(|path| uri == path)
//             })
//             .unwrap_or_default()
//             && self.name == feature.name
//     }
// }
