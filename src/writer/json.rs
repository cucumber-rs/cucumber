// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
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
    fmt::{self, Debug},
    io, mem,
    time::SystemTime,
};

use async_trait::async_trait;
use base64::Engine as _;
use derive_more::Display;
use inflector::Inflector as _;
use mime::Mime;
use once_cell::sync::Lazy;
use serde::{Serialize, Serializer};

use crate::{
    cli, event,
    feature::ExpandExamplesError,
    parser,
    writer::{
        self,
        basic::{coerce_error, trim_path},
        discard, Ext as _,
    },
    Event, World, Writer,
};

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
pub struct Json<Out: io::Write> {
    /// [`io::Write`] implementor to output [JSON][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    output: Out,

    /// Collection of [`Feature`]s to output [JSON][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    features: Vec<Feature>,

    /// [`SystemTime`] when the current [`Hook`]/[`Step`] has started.
    ///
    /// [`Hook`]: event::Hook
    started: Option<SystemTime>,

    /// [`event::Scenario::Log`]s of the current [`Hook`]/[`Step`].
    ///
    /// [`Hook`]: event::Hook
    logs: Vec<String>,
}

#[async_trait(?Send)]
impl<W: World + Debug, Out: io::Write> Writer<W> for Json<Out> {
    type Cli = cli::Empty;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
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
                self.handle_scenario_event(&f, None, &sc, ev.event, meta);
            }
            Ok((
                Cucumber::Feature(
                    f,
                    event::Feature::Rule(r, Rule::Scenario(sc, ev)),
                ),
                meta,
            )) => {
                self.handle_scenario_event(&f, Some(&r), &sc, ev.event, meta);
            }
            Ok((Cucumber::Finished, _)) => {
                self.output
                    .write_all(
                        serde_json::to_string(&self.features)
                            .unwrap_or_else(|e| {
                                panic!("Failed to serialize JSON: {e}")
                            })
                            .as_bytes(),
                    )
                    .unwrap_or_else(|e| panic!("Failed to write JSON: {e}"));
            }
            _ => {}
        }
    }
}

impl<O: io::Write> writer::NonTransforming for Json<O> {}

impl<Out: io::Write> Json<Out> {
    /// Creates a new [`Normalized`] [`Json`] [`Writer`] outputting [JSON][1]
    /// into the given `output`.
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    #[must_use]
    pub fn new<W: Debug + World>(output: Out) -> writer::Normalize<W, Self> {
        Self::raw(output).normalized()
    }

    /// Creates a new non-[`Normalized`] [`Json`] [`Writer`] outputting
    /// [JSON][1] into the given `output`, and suitable for feeding into
    /// [`tee()`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [`tee()`]: crate::WriterExt::tee
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub fn for_tee(output: Out) -> discard::Arbitrary<discard::Stats<Self>> {
        Self::raw(output)
            .discard_stats_writes()
            .discard_arbitrary_writes()
    }

    /// Creates a new raw and non-[`Normalized`] [`Json`] [`Writer`] outputting
    /// [JSON][1] into the given `output`.
    ///
    /// Use it only if you know what you're doing. Otherwise, consider using
    /// [`Json::new()`] which creates an already [`Normalized`] version of
    /// [`Json`] [`Writer`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub const fn raw(output: Out) -> Self {
        Self {
            output,
            features: vec![],
            started: None,
            logs: vec![],
        }
    }

    /// Handles the given [`event::Scenario`].
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
            Scenario::Started => {}
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
            Scenario::Log(msg) => {
                self.logs.push(msg);
            }
            Scenario::Finished => {
                self.logs.clear();
            }
        }
    }

    /// Handles the given [`event::Hook`].
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
                panic!("No `Started` event for `{hook_ty} Hook`")
            });
            meta.at
                .duration_since(started)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to compute duration between {:?} and \
                         {started:?}: {e}",
                        meta.at,
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
                embeddings: mem::take(&mut self.logs)
                    .into_iter()
                    .map(Embedding::from_log)
                    .collect(),
            },
            Hook::Failed(_, info) => HookResult {
                result: RunResult {
                    status: Status::Failed,
                    duration: duration(),
                    error_message: Some(coerce_error(&info).into_owned()),
                },
                embeddings: mem::take(&mut self.logs)
                    .into_iter()
                    .map(Embedding::from_log)
                    .collect(),
            },
        };

        let el =
            self.mut_or_insert_element(feature, rule, scenario, "scenario");
        match hook_ty {
            HookType::Before => el.before.push(res),
            HookType::After => el.after.push(res),
        }
    }

    /// Handles the given [`event::Step`].
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
                panic!("No `Started` event for `Step` '{}'", step.value)
            });
            meta.at
                .duration_since(started)
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to compute duration between {:?} and \
                         {started:?}: {e}",
                        meta.at,
                    );
                })
                .as_nanos()
        };

        let result = match event {
            event::Step::Started => {
                self.started = Some(meta.at);
                _ = self.mut_or_insert_element(feature, rule, scenario, ty);
                return;
            }
            event::Step::Passed(..) => RunResult {
                status: Status::Passed,
                duration: duration(),
                error_message: None,
            },
            event::Step::Failed(_, loc, _, err) => {
                let status = match &err {
                    event::StepError::NotFound => Status::Undefined,
                    event::StepError::AmbiguousMatch(..) => Status::Ambiguous,
                    event::StepError::Panic(..) => Status::Failed,
                };
                RunResult {
                    status,
                    duration: duration(),
                    error_message: Some(format!(
                        "{}{err}",
                        loc.map(|l| format!(
                            "Matched: {}:{}:{}\n",
                            l.path, l.line, l.column,
                        ))
                        .unwrap_or_default(),
                    )),
                }
            }
            event::Step::Skipped => RunResult {
                status: Status::Skipped,
                duration: duration(),
                error_message: None,
            },
        };

        let step = Step {
            keyword: step.keyword.clone(),
            line: step.position.line,
            name: step.value.clone(),
            hidden: false,
            result,
            embeddings: mem::take(&mut self.logs)
                .into_iter()
                .map(Embedding::from_log)
                .collect(),
        };
        let el = self.mut_or_insert_element(feature, rule, scenario, ty);
        el.steps.push(step);
    }

    /// Inserts the given `scenario`, if not present, and then returns a mutable
    /// reference to the contained value.
    fn mut_or_insert_element(
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

/// [`base64`] encoded data.
#[derive(Clone, Debug, Display, Serialize)]
#[serde(transparent)]
pub struct Base64(String);

impl Base64 {
    /// Used [`base64::engine`].
    const ENGINE: base64::engine::GeneralPurpose =
        base64::engine::general_purpose::STANDARD;

    /// Encodes `bytes` as [`base64`].
    #[must_use]
    pub fn encode(bytes: impl AsRef<[u8]>) -> Self {
        Self(Self::ENGINE.encode(bytes))
    }

    /// Decodes this [`base64`] encoded data.
    #[must_use]
    pub fn decode(&self) -> Vec<u8> {
        Self::ENGINE.decode(&self.0).unwrap_or_else(|_| {
            unreachable!(
                "the only way to construct this type is `Base64::encode`, so \
                 should contain a valid `base64` encoded `String`",
            )
        })
    }
}

/// Data embedded to [Cucumber JSON format][1] output.
///
/// [1]: https://github.com/cucumber/cucumber-json-schema
#[derive(Clone, Debug, Serialize)]
pub struct Embedding {
    /// [`base64`] encoded data.
    pub data: Base64,

    /// [`Mime`] of this [`Embedding::data`].
    #[serde(serialize_with = "serialize_display")]
    pub mime_type: Mime,

    /// Optional name of the [`Embedding`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Embedding {
    /// Creates [`Embedding`] from the provided [`event::Scenario::Log`].
    fn from_log(msg: impl AsRef<str>) -> Self {
        /// [`Mime`] of the [`event::Scenario::Log`] [`Embedding`].
        static LOG_MIME: Lazy<Mime> = Lazy::new(|| {
            "text/x.cucumber.log+plain"
                .parse()
                .unwrap_or_else(|_| unreachable!("valid MIME"))
        });

        Self {
            data: Base64::encode(msg.as_ref()),
            mime_type: LOG_MIME.clone(),
            name: None,
        }
    }
}

/// [`Serialize`]able tag of a [`gherkin::Feature`] or a [`gherkin::Scenario`].
#[derive(Clone, Debug, Serialize)]
pub struct Tag {
    /// Name of this [`Tag`].
    pub name: String,

    /// Line number of this [`Tag`] in a `.feature` file.
    ///
    /// As [`gherkin`] parser omits this info, line number is taken from
    /// [`gherkin::Feature`] or [`gherkin::Scenario`].
    pub line: usize,
}

/// Possible statuses of running [`gherkin::Step`].
#[derive(Clone, Copy, Debug, Serialize)]
pub enum Status {
    /// [`event::Step::Passed`].
    Passed,

    /// [`event::Step::Failed`] with an [`event::StepError::Panic`].
    Failed,

    /// [`event::Step::Skipped`].
    Skipped,

    /// [`event::Step::Failed`] with an [`event::StepError::AmbiguousMatch`].
    Ambiguous,

    /// [`event::Step::Failed`] with an [`event::StepError::NotFound`].
    Undefined,

    /// Never constructed and is here only to fully describe [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    Pending,
}

/// [`Serialize`]able result of running something.
#[derive(Clone, Debug, Serialize)]
pub struct RunResult {
    /// [`Status`] of this running result.
    pub status: Status,

    /// Execution time.
    ///
    /// While nowhere being documented, [`cucumber-jvm` uses nanoseconds][1].
    ///
    /// [1]: https://tinyurl.com/34wry46u#L325
    pub duration: u128,

    /// Error message of [`Status::Failed`] or [`Status::Ambiguous`] (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// [`Serialize`]able [`gherkin::Step`].
#[derive(Clone, Debug, Serialize)]
pub struct Step {
    /// [`gherkin::Step::keyword`].
    pub keyword: String,

    /// [`gherkin::Step`] line number in a `.feature` file.
    pub line: usize,

    /// [`gherkin::Step::value`].
    pub name: String,

    /// Never [`true`] and is here only to fully describe a [JSON schema][1].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,

    /// [`RunResult`] of this [`Step`].
    pub result: RunResult,

    /// [`Embedding`]s of this [`Step`].
    ///
    /// Although this field isn't present in the [JSON schema][1], all major
    /// implementations have it (see [Java], [JavaScript], [Ruby]).
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [Java]: https://bit.ly/3J66vxT
    /// [JavaScript]: https://bit.ly/41HSTAf
    /// [Ruby]: https://bit.ly/3kAJRof
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeddings: Vec<Embedding>,
}

/// [`Serialize`]able result of running a [`Before`] or [`After`] hook.
///
/// [`Before`]: event::HookType::Before
/// [`After`]: event::HookType::After
#[derive(Clone, Debug, Serialize)]
pub struct HookResult {
    /// [`RunResult`] of the hook.
    pub result: RunResult,

    /// [`Embedding`]s of this [`Hook`].
    ///
    /// Although this field isn't present in [JSON schema][1], all major
    /// implementations have it (see [Java], [JavaScript], [Ruby]).
    ///
    /// [`Hook`]: event::Hook
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [Java]: https://bit.ly/3J66vxT
    /// [JavaScript]: https://bit.ly/41HSTAf
    /// [Ruby]: https://bit.ly/3kAJRof
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeddings: Vec<Embedding>,
}

/// [`Serialize`]able [`gherkin::Background`] or [`gherkin::Scenario`].
#[derive(Clone, Debug, Serialize)]
pub struct Element {
    /// Doesn't appear in the [JSON schema][1], but present in
    /// [its generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<HookResult>,

    /// Doesn't appear in the [JSON schema][1], but present in
    /// [its generated test cases][2].
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: https://github.com/cucumber/cucumber-json-testdata-generator
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<HookResult>,

    /// [`gherkin::Scenario::keyword`].
    pub keyword: String,

    /// Type of this [`Element`].
    ///
    /// Only set to `background` or `scenario`, but [JSON schema][1] doesn't
    /// constraint only to those values, so maybe a subject to change.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    pub r#type: &'static str,

    /// Identifier of this [`Element`]. Doesn't have to be unique.
    pub id: String,

    /// [`gherkin::Scenario`] line number inside a `.feature` file.
    pub line: usize,

    /// [`gherkin::Scenario::name`], optionally prepended with a
    /// [`gherkin::Rule::name`].
    ///
    /// This is done because [JSON schema][1] doesn't support [`gherkin::Rule`]s
    /// at the moment.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    pub name: String,

    /// [`gherkin::Scenario::tags`].
    pub tags: Vec<Tag>,

    /// [`gherkin::Scenario`]'s [`Step`]s.
    pub steps: Vec<Step>,
}

impl Element {
    /// Creates a new [`Element`] out of the given values.
    fn new(
        feature: &gherkin::Feature,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        ty: &'static str,
    ) -> Self {
        Self {
            after: vec![],
            before: vec![],
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
                scenario.name.clone(),
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

/// [`Serialize`]able [`gherkin::Feature`].
#[derive(Clone, Debug, Serialize)]
pub struct Feature {
    /// [`gherkin::Feature::path`].
    pub uri: Option<String>,

    /// [`gherkin::Feature::keyword`].
    pub keyword: String,

    /// [`gherkin::Feature::name`].
    pub name: String,

    /// [`gherkin::Feature::tags`].
    pub tags: Vec<Tag>,

    /// [`gherkin::Feature`]'s [`Element`]s.
    pub elements: Vec<Element>,
}

impl Feature {
    /// Creates a new [`Feature`] out of the given [`gherkin::Feature`].
    fn new(feature: &gherkin::Feature) -> Self {
        Self {
            uri: feature
                .path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
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
            elements: vec![],
        }
    }

    /// Creates a new [`Feature`] from the given [`ExpandExamplesError`].
    fn example_expansion_err(err: &ExpandExamplesError) -> Self {
        Self {
            uri: err
                .path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .map(str::to_owned),
            keyword: String::new(),
            name: String::new(),
            tags: vec![],
            elements: vec![Element {
                after: vec![],
                before: vec![],
                keyword: String::new(),
                r#type: "scenario",
                id: format!(
                    "failed-to-expand-examples{}",
                    err.path
                        .as_ref()
                        .and_then(|p| p.to_str().map(trim_path))
                        .unwrap_or_default(),
                ),
                line: 0,
                name: String::new(),
                tags: vec![],
                steps: vec![Step {
                    keyword: String::new(),
                    line: err.pos.line,
                    name: "scenario".into(),
                    hidden: false,
                    result: RunResult {
                        status: Status::Failed,
                        duration: 0,
                        error_message: Some(err.to_string()),
                    },
                    embeddings: vec![],
                }],
            }],
        }
    }

    /// Creates a new [`Feature`] from the given [`gherkin::ParseFileError`].
    fn parsing_err(err: &gherkin::ParseFileError) -> Self {
        let path = match err {
            gherkin::ParseFileError::Reading { path, .. }
            | gherkin::ParseFileError::Parsing { path, .. } => path,
        }
        .to_str()
        .map(trim_path)
        .map(str::to_owned);

        Self {
            uri: path.clone(),
            keyword: String::new(),
            name: String::new(),
            tags: vec![],
            elements: vec![Element {
                after: vec![],
                before: vec![],
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
                    name: "scenario".into(),
                    hidden: false,
                    result: RunResult {
                        status: Status::Failed,
                        duration: 0,
                        error_message: Some(err.to_string()),
                    },
                    embeddings: vec![],
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
                    .and_then(|p| p.to_str().map(trim_path))
                    .map(|path| uri == path)
            })
            .unwrap_or_default()
            && self.name == feature.name
    }
}

/// Helper to use `#[serde(serialize_with = "serialize_display")]` with any type
/// implementing [`fmt::Display`].
fn serialize_display<T, S>(display: &T, ser: S) -> Result<S::Ok, S::Error>
where
    T: fmt::Display,
    S: Serializer,
{
    format_args!("{display}").serialize(ser)
}
