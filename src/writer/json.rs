//! [JSON schema report][1] [`Writer`] implementation.
//!
//! [1]: https://github.com/cucumber/cucumber-json-schema

use std::{fmt::Debug, io, time::SystemTime};

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

/// [JSON schema][1] [`Writer`] implementation outputting to an [`io::Write`]
/// implementor.
///
/// Should be wrapped into [`writer::Normalized`] to work correctly, otherwise
/// will panic in runtime as won't be able to form correct [JSON schema][1].
///
/// [1]: https://github.com/cucumber/cucumber-json-schema
#[derive(Clone, Debug)]
pub struct Json<Out: io::Write> {
    /// [`io::Write`] implementor to output [JSON schema][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    output: Out,

    /// Collection of [`Feature`]s to output [JSON schema][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    features: Vec<Feature>,

    /// [`SystemTime`] when the current [`Hook`]/[`Step`] has started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Hook`]: event::Hook
    started: Option<SystemTime>,
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
    /// Creates a new normalized [`Json`] [`Writer`] outputting [JSON schema][1]
    /// into the given `output`.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
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
    /// [JSON schema][1] from unordered [`Cucumber` events][2].
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
            started: None,
        }
    }

    /// Handles [`event::Cucumber`].
    fn handle_event<W>(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
    ) {
        use event::{Cucumber, Rule};

        match event.map(event::Event::split) {
            Err(parser::Error::Parsing(e)) => {
                let feature = Feature::parsing_err(&e);
                self.features.push(feature);
            }
            Err(parser::Error::ExampleExpansion(e)) => {
                let feature = Feature::example_expansion_err(&e);
                self.features.push(feature);
            }
            Ok((
                Cucumber::Feature(f, event::Feature::Scenario(sc, ev)),
                meta,
            )) => {
                self.handle_scenario_event(&f, None, &sc, ev, meta);
            }
            Ok((
                Cucumber::Feature(
                    f,
                    event::Feature::Rule(r, Rule::Scenario(sc, ev)),
                ),
                meta,
            )) => {
                self.handle_scenario_event(&f, Some(&r), &sc, ev, meta);
            }
            Ok((Cucumber::Finished, _)) => {
                self.output
                    .write_all(
                        serde_json::to_string(&self.features)
                            .unwrap_or_else(|e| {
                                panic!("Failed to serialize: {}", e)
                            })
                            .as_bytes(),
                    )
                    .unwrap_or_else(|e| panic!("Failed to write: {}", e));
            }
            _ => {}
        }
    }

    /// Handles [`event::Scenario`].
    fn handle_scenario_event<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ev: event::Scenario<W>,
        meta: event::Metadata,
    ) {
        use event::Scenario;

        match ev {
            Scenario::Hook(ty, ev) => {
                self.handle_hook_event(feature, rule, scenario, ty, ev, meta);
            }
            Scenario::Background(st, ev) => {
                self.handle_step_event(
                    feature,
                    rule,
                    scenario,
                    "background",
                    &st,
                    ev,
                    meta,
                );
            }
            Scenario::Step(st, ev) => {
                self.handle_step_event(
                    feature, rule, scenario, "scenario", &st, ev, meta,
                );
            }
            Scenario::Started | Scenario::Finished => {}
        }
    }

    /// Handles [`event::Hook`].
    fn handle_hook_event<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        hook_ty: event::HookType,
        event: event::Hook<W>,
        meta: event::Metadata,
    ) {
        use event::{Hook, HookType};

        let mut duration = || {
            let started = self.started.take().unwrap_or_else(|| {
                panic!("No Started event for {} Hook", hook_ty)
            });
            meta.at
                .duration_since(started)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to compute duration between {:?} and {:?}: {}",
                        meta.at, started, e,
                    );
                })
                .as_nanos()
        };

        let res = match event {
            Hook::Started => {
                self.started = Some(meta.at);
                return;
            }
            Hook::Passed => HookResult {
                result: RunResult {
                    status: Status::Passed,
                    duration: duration(),
                    error_message: None,
                },
            },
            Hook::Failed(_, info) => HookResult {
                result: RunResult {
                    status: Status::Failed,
                    duration: duration(),
                    error_message: Some(coerce_error(&info).into_owned()),
                },
            },
        };

        let el =
            self.get_or_insert_element(feature, rule, scenario, "scenario");
        match hook_ty {
            HookType::Before => el.before.push(res),
            HookType::After => el.after.push(res),
        }
    }

    /// Handles [`event::Step`].
    #[allow(clippy::too_many_arguments)]
    fn handle_step_event<W>(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
        step: &gherkin::Step,
        event: event::Step<W>,
        meta: event::Metadata,
    ) {
        let mut duration = || {
            let started = self.started.take().unwrap_or_else(|| {
                panic!("No Started event for Step '{}'", step.value)
            });
            meta.at
                .duration_since(started)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to compute duration between {:?} and {:?}: {}",
                        meta.at, started, e,
                    );
                })
                .as_nanos()
        };

        let result = match event {
            event::Step::Started => {
                self.started = Some(meta.at);
                let _ = self.get_or_insert_element(feature, rule, scenario, ty);
                return;
            }
            event::Step::Passed(..) => RunResult {
                status: Status::Passed,
                duration: duration(),
                error_message: None,
            },
            event::Step::Failed(_, _, err) => match err {
                event::StepError::AmbiguousMatch(err) => RunResult {
                    status: Status::Ambiguous,
                    duration: duration(),
                    error_message: Some(format!("{}", err)),
                },
                event::StepError::Panic(info) => RunResult {
                    status: Status::Failed,
                    duration: duration(),
                    error_message: Some(coerce_error(&info).into_owned()),
                },
            },
            event::Step::Skipped => RunResult {
                status: Status::Skipped,
                duration: duration(),
                error_message: None,
            },
        };

        let el = self.get_or_insert_element(feature, rule, scenario, ty);
        el.steps.push(Step {
            keyword: step.keyword.clone(),
            line: step.position.line,
            name: step.value.clone(),
            hidden: false,
            result,
        });
    }

    /// Inserts `scenario`, if not present, then returns a mutable reference to
    /// the contained value.
    fn get_or_insert_element(
        &mut self,
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> &mut Element {
        let f_pos = self
            .features
            .iter()
            .position(|f| f == feature)
            .unwrap_or_else(|| {
                self.features.push(Feature::new(feature));
                self.features.len() - 1
            });
        let f = self
            .features
            .get_mut(f_pos)
            .unwrap_or_else(|| unreachable!());
        let el_pos = f
            .elements
            .iter()
            .position(|el| {
                el.name
                    == format!(
                        "{}{}",
                        rule.map(|r| format!("{} ", r.name))
                            .unwrap_or_default(),
                        scenario.name,
                    )
                    && el.line == scenario.position.line
                    && el.r#type == ty
            })
            .unwrap_or_else(|| {
                f.elements.push(Element::new(feature, rule, scenario, ty));
                f.elements.len() - 1
            });
        f.elements.get_mut(el_pos).unwrap_or_else(|| unreachable!())
    }
}

/// [`gherkin::Feature`] or [`gherkin::Scenario`] tag.
#[derive(Clone, Debug, Serialize)]
pub struct Tag {
    /// [`Tag`] name.
    pub name: String,

    /// Line number.
    ///
    /// As [`gherkin`] parser omits this info, line number is taken from
    /// [`gherkin::Feature`] or [`gherkin::Scenario`].
    pub line: usize,
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

/// Run result.
#[derive(Clone, Debug, Serialize)]
pub struct RunResult {
    /// [`Status`].
    pub status: Status,

    /// Execution time.
    ///
    /// While nowhere to be documented, `cucumber-jvm` uses nanoseconds.
    /// Source: <https://bit.ly/3onkLXJ>
    pub duration: u128,

    /// Error message.
    ///
    /// Present only if [`Status::Failed`] or [`Status::Ambiguous`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// [`gherkin::Step`].
#[derive(Clone, Debug, Serialize)]
pub struct Step {
    /// [`gherkin::Step::keyword`].
    pub keyword: String,

    /// [`gherkin::Step`] line number in `.feature` file.
    pub line: usize,

    /// [`gherkin::Step::value`].
    pub name: String,

    /// Never [`true`] and is here only to fully describe [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,

    /// [`gherkin::Step`] run result.
    pub result: RunResult,
}

/// [`Before`] or [`After`] hook run result.
///
/// [`Before`]: event::HookType::Before
/// [`After`]: event::HookType::After
#[derive(Clone, Debug, Serialize)]
pub struct HookResult {
    /// [`Before`] or [`After`] hook run result.
    ///
    /// [`Before`]: event::HookType::Before
    /// [`After`]: event::HookType::After
    pub result: RunResult,
}

/// [`gherkin::Background`] or [`gherkin::Scenario`].
#[derive(Clone, Debug, Serialize)]
pub struct Element {
    /// Doesn't appear in the [JSON schema][1], but present in
    /// [generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<HookResult>,

    /// Doesn't appear in the [JSON schema][1], but present in
    /// [generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<HookResult>,

    /// [`gherkin::Scenario::keyword`].
    pub keyword: String,

    /// [`Element`] type.
    ///
    /// Only set to `background` or `scenario`, but [JSON schema][1] doesn't
    /// constraint only to those values, so maybe a subject to change.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    pub r#type: &'static str,

    /// [`Element`] identifier. Doesn't have to be unique.
    pub id: String,

    /// [`gherkin::Scenario`] line number inside `.feature` file.
    pub line: usize,

    /// [`gherkin::Scenario::name`] maybe prepended with
    /// [`gherkin::Rule::name`]. This is done because [JSON schema][1] doesn't
    /// have [`gherkin::Rule`].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    pub name: String,

    /// [`gherkin::Scenario::tags`].
    pub tags: Vec<Tag>,

    /// [`gherkin::Scenario`] [`Step`]s.
    pub steps: Vec<Step>,
}

impl Element {
    /// Creates a new [`Element`].
    fn new(
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> Self {
        Self {
            after: Vec::new(),
            before: Vec::new(),
            keyword: (ty == "background")
                .then(|| feature.background.as_ref().map(|bg| &bg.keyword))
                .flatten()
                .unwrap_or(&scenario.keyword)
                .clone(),
            r#type: ty,
            id: format!(
                "{}{}/{}",
                feature.name.to_kebab_case(),
                rule.map(|r| format!("/{}", r.name.to_kebab_case()))
                    .unwrap_or_default(),
                scenario.name.to_kebab_case(),
            ),
            line: scenario.position.line,
            name: format!(
                "{}{}",
                rule.map(|r| format!("{} ", r.name)).unwrap_or_default(),
                scenario.name.clone()
            ),
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
    pub uri: Option<String>,

    /// [`gherkin::Feature::keyword`].
    pub keyword: String,

    /// [`gherkin::Feature::name`].
    pub name: String,

    /// [`gherkin::Feature::tags`]
    pub tags: Vec<Tag>,

    /// [`gherkin::Feature`] [`Element`]s.
    pub elements: Vec<Element>,
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
            keyword: feature.keyword.clone(),
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
                    result: RunResult {
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
                    result: RunResult {
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
