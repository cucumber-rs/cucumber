//! [JSON schema report][1] [`Writer`] implementation.
//!
//! [1]: https://github.com/cucumber/cucumber-json-schema

// TODO: add failing after hook

use std::{
    fmt::Debug,
    io,
    time::{Duration, SystemTime},
};

use async_trait::async_trait;
use inflector::Inflector as _;
use serde::Serialize;

use crate::{
    cli, event,
    feature::ExpandExamplesError,
    parser,
    writer::{self, basic::coerce_error},
    Event, World, Writer, WriterExt as _,
};

/// [JSON schema report][1] [`Writer`] implementation outputting to an
/// [`io::Write`] implementor.
///
/// Should be wrapped into [`writer::Normalized`] to work correctly, otherwise
/// will panic in runtime as won't be able to form correct
/// [JSON `testsuite`s][1].
///
/// [1]: https://github.com/cucumber/cucumber-json-schema
#[derive(Clone, Debug)]
pub struct Json<Out: io::Write> {
    /// [`io::Write`] implementor to output XML report into.
    output: Out,

    /// Collection of [`Feature`]s to output [JSON report][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    features: Vec<Feature>,

    /// [`SystemTime`] when the current [`Step`] has started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    step_started: Option<SystemTime>,
}

#[async_trait(?Send)]
impl<W: World + Debug, Out: io::Write> Writer<W> for Json<Out> {
    type Cli = cli::Empty;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        self.handle_event(event);
    }
}

impl<Out: io::Write> Json<Out> {
    /// Creates a new normalized [`Json`] [`Writer`] outputting XML report into
    /// the given `output`.
    #[must_use]
    pub fn new<W: Debug + World>(output: Out) -> writer::Normalized<W, Self> {
        Self::raw(output).normalized()
    }

    /// Creates a new raw and unnormalized [`Json`] [`Writer`] outputting report
    /// into the given `output`.
    ///
    /// # Warning
    ///
    /// It may panic in runtime as won't be able to form correct
    /// [Json `testsuite`s][1] from unordered [`Cucumber` events][2].
    ///
    /// Use it only if you know what you're doing. Otherwise, consider using
    /// [`Json::new()`] which creates an already [`Normalized`] version of
    /// [`Json`] [`Writer`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub fn raw(output: Out) -> Self {
        Self {
            output,
            features: Vec::new(),
            step_started: None,
        }
    }

    fn handle_event<W>(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
    ) {
        use event::{Cucumber, Feature, Rule};

        match event.map(event::Event::split) {
            Ok((Cucumber::Feature(f, Feature::Scenario(sc, ev)), meta)) => {
                self.handle_scenario_event(&f, None, &sc, ev, meta);
            }
            Ok((
                Cucumber::Feature(f, Feature::Rule(r, Rule::Scenario(sc, ev))),
                meta,
            )) => {
                self.handle_scenario_event(&f, Some(&r), &sc, ev, meta);
            }
            Err(_) => {
                // add failure
            }
            _ => {}
        }
    }

    fn handle_scenario_event<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: event::Scenario<W>,
        meta: event::Metadata,
    ) {
        match ev {
            event::Scenario::Started => {
                self.get_or_insert_element(feature, rule, scenario, "scenario");
            }
            event::Scenario::Hook(_, event::Hook::Started) => {
                self.step_started = Some(meta.at);
            }
            event::Scenario::Hook(ty, ev) => {
                self.handle_hook_event(feature, scenario, ty, ev, meta);
            }
            event::Scenario::Background(_, event::Step::Started) => {
                self.set_scenario_type(feature, rule, scenario, "background");
            }
            event::Scenario::Background(
                st,
                ev @ (event::Step::Passed(..)
                | event::Step::Skipped
                | event::Step::Failed(..)),
            ) => {
                let el = self.get_or_insert_element(
                    feature,
                    rule,
                    scenario,
                    "background",
                );
                let duration = meta
                    .at
                    .duration_since(self.step_started.take().unwrap())
                    .unwrap();
                // TODO: insert 'background', not 'scenario' type
                el.steps.push(Step::new(&st, &ev, duration));
            }
            event::Scenario::Step(_, event::Step::Started) => {
                self.step_started = Some(meta.at);
                self.set_scenario_type(feature, rule, scenario, "scenario");
            }
            event::Scenario::Step(
                st,
                ev @ (event::Step::Passed(..)
                | event::Step::Skipped
                | event::Step::Failed(..)),
            ) => {
                let el = self
                    .get_or_insert_element(feature, rule, scenario, "scenario");
                let duration = meta
                    .at
                    .duration_since(self.step_started.take().unwrap())
                    .unwrap();
                el.steps.push(Step::new(&st, &ev, duration));
            }
        }
    }

    fn handle_hook_event<W>(
        &mut self,
        feature: &gherkin::Feature,
        scenario: &gherkin::Scenario,
        hook_ty: event::HookType,
        event: event::Hook<W>,
        meta: event::Metadata,
    ) {
        match event {
            event::Hook::Started => {
                self.step_started = Some(meta.at);
            }
            ev => {
                let f =
                    self.features.iter_mut().find(|&f| *f == *feature).unwrap();
                let el = f
                    .elements
                    .iter_mut()
                    .find(|el| el.name == scenario.name)
                    .unwrap();
                let duration = meta
                    .at
                    .duration_since(self.step_started.take().unwrap())
                    .unwrap()
                    .as_nanos();
                let res = match ev {
                    event::Hook::Started => unreachable!(),
                    event::Hook::Passed => HookResult {
                        result: StepResult {
                            status: Status::Passed,
                            duration,
                            error_message: None,
                        },
                    },
                    event::Hook::Failed(_, info) => HookResult {
                        result: StepResult {
                            status: Status::Failed,
                            duration,
                            error_message: Some(
                                coerce_error(&info).into_owned(),
                            ),
                        },
                    },
                };
                match hook_ty {
                    event::HookType::Before => el.before.push(res),
                    event::HookType::After => el.after.push(res),
                }
            }
        }
    }

    fn get_or_insert_element(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> &mut Element {
        let f = self
            .features
            .iter_mut()
            .find(|&f| *f == *feature)
            .unwrap_or_else(|| {
                self.features.push(Feature::new(feature));
                self.features.last_mut().unwrap()
            });
        f.elements
            .iter_mut()
            .find(|el| *el.name == scenario.name && el.r#type == ty)
            .unwrap_or_else(|| {
                f.elements.push(Element::new(feature, rule, scenario, ty));
                f.elements.last_mut().unwrap()
            })
    }

    fn set_scenario_type(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) {
        let f = self
            .features
            .iter_mut()
            .find(|&f| *f == *feature)
            .unwrap_or_else(|| {
                self.features.push(Feature::new(feature));
                self.features.last_mut().unwrap()
            });
        let el = f
            .elements
            .iter_mut()
            .find(|&el| *el.name == scenario.name && el.r#type == "scenario")
            .unwrap_or_else(|| {
                f.elements.push(Element::new(feature, rule, scenario, ty));
                f.elements.last_mut().unwrap()
            });
        el.r#type = ty;
    }
}

/// [`gherkin::Feature`] or [`gherkin::Scenario`] tag.
#[derive(Clone, Debug, Serialize)]
pub struct Tag {
    /// [`Tag`] name.
    name: String,

    /// Line number.
    ///
    /// As [`gherkin`] parser omits this info, line number is taken from
    /// [`gherkin::Feature`] or [`gherkin::Scenario`].
    line: usize,
}

/// [`gherkin::Step`] run status.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum Status {
    /// [`event::Step::Passed`].
    Passed,

    /// [`event::Step::Failed`] with [`event::StepError::Panic`].
    Failed,

    /// [`event::Step::Skipped`].
    Skipped,

    /// [`event::Step::Failed`] with [`event::StepError::AmbiguousMatch`].
    Ambiguous,

    /// Never constructed and is here only to fully describe [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    Undefined,

    /// Never constructed and is here only to fully describe [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    Pending,
}

/// [`gherkin::Step`] run result.
#[derive(Clone, Debug, Serialize)]
pub struct StepResult {
    /// [`gherkin::Step`] [`Status`].
    status: Status,

    /// [`gherkin::Step`] execution time.
    ///
    /// While nowhere to be documented, `cucumber-jvm` uses nanoseconds.
    /// Source: https://bit.ly/3onkLXJ
    duration: u128,

    /// Error message.
    ///
    /// Present only if [`Status::Failed`] or [`Status::Ambiguous`].
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<String>,
}

/// [`gherkin::Step`].
#[derive(Clone, Debug, Serialize)]
pub struct Step {
    /// [`gherkin::Step::keyword`] with trailing whitespace.
    ///
    /// Source: https://bit.ly/3c2q5tK
    keyword: String,

    /// [`gherkin::Step`] line number in `.feature` file.
    line: usize,

    /// [`gherkin::Step::name`].
    name: String,

    /// Never [`true`] and is here only to fully describe [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    hidden: bool,

    /// [`gherkin::Step`] run result.
    result: StepResult,
}

impl Step {
    /// Creates a new [`Step`].
    fn new<W>(
        step: &gherkin::Step,
        event: &event::Step<W>,
        duration: Duration,
    ) -> Self {
        Self {
            keyword: format!("{} ", step.keyword),
            line: step.position.line,
            name: step.value.clone(),
            hidden: false,
            result: match event {
                // TODO: maybe we should handle this differently
                event::Step::Started => panic!(""),
                event::Step::Passed(..) => StepResult {
                    status: Status::Passed,
                    duration: duration.as_nanos(),
                    error_message: None,
                },
                event::Step::Failed(_, _, err) => match err {
                    event::StepError::AmbiguousMatch(err) => StepResult {
                        status: Status::Ambiguous,
                        duration: duration.as_nanos(),
                        error_message: Some(format!("{}", err)),
                    },
                    event::StepError::Panic(info) => StepResult {
                        status: Status::Failed,
                        duration: duration.as_micros(),
                        error_message: Some(coerce_error(info).into_owned()),
                    },
                },
                event::Step::Skipped => StepResult {
                    status: Status::Skipped,
                    duration: duration.as_nanos(),
                    error_message: None,
                },
            },
        }
    }
}

/// TODO
#[derive(Clone, Debug, Serialize)]
pub struct HookResult {
    /// TODO
    result: StepResult,
}

/// [`gherkin::Background`] or [`gherkin::Scenario`] maybe prepended with
/// [`gherkin::Rule::name`]. This is done because [JSON schema][1] doesn't have
/// [`gherkin::Rule`].
///
/// [1]: https://github.com/cucumber/cucumber-json-schema
#[derive(Clone, Debug, Serialize)]
pub struct Element {
    /// Doesn't appear in the [JSON schema][1], but present in
    /// [generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    after: Vec<HookResult>,

    /// Doesn't appear in the [JSON schema][1], but present in
    /// [generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    before: Vec<HookResult>,

    /// [`gherkin::Scenario::keyword`] with trailing whitespace.
    ///
    /// Source: https://bit.ly/3c2q5tK
    keyword: String,

    /// [`Element`] type.
    ///
    /// Only set to `background` or `scenario`, but [JSON schema][1] doesn't
    /// constraint only to those values, so maybe a subject to change.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    r#type: &'static str,

    /// [`Element`] identifier. Doesn't have to be unique.
    id: String,

    /// [`gherkin::Scenario`] line number inside `.feature` file.
    line: usize,

    /// [`gherkin::Scenario::name`].
    name: String,

    /// [`gherkin::Scenario::tags`].
    tags: Vec<Tag>,

    /// [`gherkin::Scenario`] [`Step`]s.
    steps: Vec<Step>,
}

impl Element {
    fn new(
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> Self {
        Self {
            after: Vec::new(),
            before: Vec::new(),
            keyword: format!("{} ", scenario.keyword),
            r#type: ty,
            id: format!(
                "{}{}/{}",
                feature.name,
                rule.map(|r| format!("/{}", r.name)).unwrap_or_default(),
                scenario.name,
            ),
            line: scenario.position.line,
            name: scenario.name.clone(),
            tags: scenario
                .tags
                .iter()
                .map(|t| Tag {
                    name: t.clone(),
                    line: scenario.position.line,
                })
                .collect(),
            steps: vec![],
        }
    }
}

/// [`gherkin::Feature`].
#[derive(Clone, Debug, Serialize)]
pub struct Feature {
    /// [`gherkin::Feature::path`].
    uri: Option<String>,

    /// [`gherkin::Feature::keyword`] with trailing whitespace.
    ///
    /// Source: https://bit.ly/3c2q5tK
    keyword: String,

    /// [`gherkin::Feature::name`].
    name: String,

    /// [`gherkin::Feature::tags`]
    tags: Vec<Tag>,

    /// [`gherkin::Feature`] [`Element`]s.
    elements: Vec<Element>,
}

impl Feature {
    /// Creates a new [`Feature`].
    fn new(feature: &gherkin::Feature) -> Self {
        Self {
            uri: feature
                .path
                .as_ref()
                .and_then(|p| p.to_str())
                .map(str::to_owned),
            keyword: format!("{} ", feature.keyword),
            name: feature.name.clone(),
            tags: feature
                .tags
                .iter()
                .map(|tag| Tag {
                    name: tag.clone(),
                    line: feature.position.line,
                })
                .collect(),
            elements: Vec::new(),
        }
    }

    /// Creates a new [`Feature`] from [`ExpandExamplesError`].
    fn example_expansion_err(err: &ExpandExamplesError) -> Self {
        Self {
            uri: err
                .path
                .as_ref()
                .and_then(|p| p.to_str())
                .map(str::to_owned),
            keyword: String::new(),
            name: String::new(),
            tags: vec![],
            elements: vec![Element {
                after: Vec::new(),
                before: Vec::new(),
                keyword: String::new(),
                r#type: "scenario",
                id: format!(
                    "failed-to-expand-examples{}",
                    err.path
                        .as_ref()
                        .and_then(|p| p.to_str())
                        .unwrap_or_default(),
                ),
                line: 0,
                name: String::new(),
                tags: vec![],
                steps: vec![Step {
                    keyword: String::new(),
                    line: err.pos.line,
                    name: "scenario".to_owned(),
                    hidden: false,
                    result: StepResult {
                        status: Status::Failed,
                        duration: 0,
                        error_message: Some(format!("{}", err)),
                    },
                }],
            }],
        }
    }

    /// Creates a new [`Feature`] from [`gherkin::ParseFileError`].
    fn parsing_err(err: &gherkin::ParseFileError) -> Self {
        let path = match err {
            gherkin::ParseFileError::Reading { path, .. }
            | gherkin::ParseFileError::Parsing { path, .. } => path,
        }
        .to_str()
        .map(str::to_owned);

        Self {
            uri: path.clone(),
            keyword: String::new(),
            name: String::new(),
            tags: vec![],
            elements: vec![Element {
                after: Vec::new(),
                before: Vec::new(),
                keyword: String::new(),
                r#type: "scenario",
                id: format!(
                    "failed-to-parse{}",
                    path.as_deref().unwrap_or_default(),
                ),
                line: 0,
                name: String::new(),
                tags: vec![],
                steps: vec![Step {
                    keyword: String::new(),
                    line: 0,
                    name: "scenario".to_owned(),
                    hidden: false,
                    result: StepResult {
                        status: Status::Failed,
                        duration: 0,
                        error_message: Some(format!("{}", err)),
                    },
                }],
            }],
        }
    }
}

impl PartialEq<gherkin::Feature> for Feature {
    fn eq(&self, feature: &gherkin::Feature) -> bool {
        self.uri
            .as_ref()
            .and_then(|uri| {
                feature
                    .path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .map(|path| uri == path)
            })
            .unwrap_or_default()
            && self.name == feature.name
    }
}
