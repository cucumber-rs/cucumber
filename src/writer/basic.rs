// Copyright (c) 2018-2023  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Default [`Writer`] implementation.

use std::{
    borrow::Cow,
    cmp, env,
    fmt::{Debug, Display},
    io,
    str::FromStr,
};

use async_trait::async_trait;
use derive_more::{Deref, DerefMut};
use itertools::Itertools as _;
use once_cell::sync::Lazy;
use regex::CaptureLocations;
use smart_default::SmartDefault;

use crate::{
    cli::Colored,
    event::{self, Info, Retries},
    parser, step,
    writer::{
        self,
        out::{Styles, WriteStrExt as _},
        Ext as _, Verbosity,
    },
    Event, World, Writer,
};

/// CLI options of a [`Basic`] [`Writer`].
#[derive(clap::Args, Clone, Copy, Debug, SmartDefault)]
#[group(skip)]
pub struct Cli {
    /// Verbosity of an output.
    ///
    /// `-v` is default verbosity, `-vv` additionally outputs world on failed
    /// steps, `-vvv` additionally outputs step's doc string (if present).
    #[arg(short, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Coloring policy for a console output.
    #[arg(
        long,
        value_name = "auto|always|never",
        default_value = "auto",
        global = true
    )]
    #[default(Coloring::Auto)]
    pub color: Coloring,
}

impl Colored for Cli {
    fn coloring(&self) -> Coloring {
        self.color
    }
}

/// Possible policies of a [`console`] output coloring.
#[derive(Clone, Copy, Debug)]
pub enum Coloring {
    /// Letting [`console::colors_enabled()`] to decide, whether output should
    /// be colored.
    Auto,

    /// Forcing of a colored output.
    Always,

    /// Forcing of a non-colored output.
    Never,
}

impl FromStr for Coloring {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err("possible options: auto, always, never"),
        }
    }
}

/// Default [`Writer`] implementation outputting to an [`io::Write`] implementor
/// ([`io::Stdout`] by default).
///
/// Pretty-prints with colors if terminal was successfully detected, otherwise
/// has simple output. Useful for running tests with CI tools.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into
/// a [`writer::Normalize`], otherwise will produce output [`Event`]s in a
/// broken order.
///
/// [`Normalized`]: writer::Normalized
/// [`Runner`]: crate::runner::Runner
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct Basic<Out: io::Write = io::Stdout> {
    /// [`io::Write`] implementor to write the output into.
    #[deref]
    #[deref_mut]
    output: Out,

    /// [`Styles`] for terminal output.
    styles: Styles,

    /// Current indentation that events are outputted with.
    indent: usize,

    /// Number of lines to clear.
    lines_to_clear: usize,

    /// Buffer to be re-output after [`clear_last_lines_if_term_present()`][0].
    ///
    /// [0]: Self::clear_last_lines_if_term_present
    re_output_after_clear: String,

    /// [`Verbosity`] of this [`Writer`].
    verbosity: Verbosity,
}

#[async_trait(?Send)]
impl<W, Out> Writer<W> for Basic<Out>
where
    W: World + Debug,
    Out: io::Write,
{
    type Cli = Cli;

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        opts: &Self::Cli,
    ) {
        use event::{Cucumber, Feature};

        self.apply_cli(*opts);

        match ev.map(Event::into_inner) {
            Err(err) => self.parsing_failed(&err),
            Ok(
                Cucumber::Started
                | Cucumber::ParsingFinished { .. }
                | Cucumber::Finished,
            ) => Ok(()),
            Ok(Cucumber::Feature(f, ev)) => match ev {
                Feature::Started => self.feature_started(&f),
                Feature::Scenario(sc, ev) => self.scenario(&f, &sc, &ev),
                Feature::Rule(r, ev) => self.rule(&f, &r, ev),
                Feature::Finished => Ok(()),
            },
        }
        .unwrap_or_else(|e| panic!("Failed to write into terminal: {e}"));
    }
}

#[async_trait(?Send)]
impl<'val, W, Val, Out> writer::Arbitrary<'val, W, Val> for Basic<Out>
where
    W: World + Debug,
    Val: AsRef<str> + 'val,
    Out: io::Write,
{
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.write_line(val.as_ref())
            .unwrap_or_else(|e| panic!("Failed to write: {e}"));
    }
}

impl<O: io::Write> writer::NonTransforming for Basic<O> {}

impl Basic {
    /// Creates a new [`Normalized`] [`Basic`] [`Writer`] outputting to
    /// [`io::Stdout`].
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn stdout<W>() -> writer::Normalize<W, Self> {
        Self::new(io::stdout(), Coloring::Auto, Verbosity::Default)
    }
}

impl<Out: io::Write> Basic<Out> {
    /// Creates a new [`Normalized`] [`Basic`] [`Writer`] outputting to the
    /// given `output`.
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn new<W>(
        output: Out,
        color: Coloring,
        verbosity: impl Into<Verbosity>,
    ) -> writer::Normalize<W, Self> {
        Self::raw(output, color, verbosity).normalized()
    }

    /// Creates a new non-[`Normalized`] [`Basic`] [`Writer`] outputting to the
    /// given `output`.
    ///
    /// Use it only if you know what you're doing. Otherwise, consider using
    /// [`Basic::new()`] which creates an already [`Normalized`] version of a
    /// [`Basic`] [`Writer`].
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn raw(
        output: Out,
        color: Coloring,
        verbosity: impl Into<Verbosity>,
    ) -> Self {
        let mut basic = Self {
            output,
            styles: Styles::new(),
            indent: 0,
            lines_to_clear: 0,
            re_output_after_clear: String::new(),
            verbosity: verbosity.into(),
        };
        basic.apply_cli(Cli {
            verbose: u8::from(basic.verbosity) + 1,
            color,
        });
        basic
    }

    /// Applies the given [`Cli`] options to this [`Basic`] [`Writer`].
    pub fn apply_cli(&mut self, cli: Cli) {
        match cli.verbose {
            0 => {}
            1 => self.verbosity = Verbosity::Default,
            2 => self.verbosity = Verbosity::ShowWorld,
            _ => self.verbosity = Verbosity::ShowWorldAndDocString,
        };
        self.styles.apply_coloring(cli.color);
    }

    /// Clears last `n` lines if [`Coloring`] is enabled.
    fn clear_last_lines_if_term_present(&mut self) -> io::Result<()> {
        if self.styles.is_present && self.lines_to_clear > 0 {
            self.output.clear_last_lines(self.lines_to_clear)?;
            self.output.write_str(&self.re_output_after_clear)?;
            self.re_output_after_clear.clear();
            self.lines_to_clear = 0;
        }
        Ok(())
    }

    /// Outputs the parsing `error` encountered while parsing some [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    pub(crate) fn parsing_failed(
        &mut self,
        error: impl Display,
    ) -> io::Result<()> {
        self.output
            .write_line(&self.styles.err(format!("Failed to parse: {error}")))
    }

    /// Outputs the [started] [`Feature`].
    ///
    /// [started]: event::Feature::Started
    /// [`Feature`]: gherkin::Feature
    pub(crate) fn feature_started(
        &mut self,
        feature: &gherkin::Feature,
    ) -> io::Result<()> {
        let out = format!("{}: {}", feature.keyword, feature.name);
        self.output.write_line(&self.styles.ok(out))
    }

    /// Outputs the [`Rule`]'s [started]/[scenario]/[finished] event.
    ///
    /// [finished]: event::Rule::Finished
    /// [scenario]: event::Rule::Scenario
    /// [started]: event::Rule::Started
    /// [`Rule`]: gherkin::Rule
    pub(crate) fn rule<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        rule: &gherkin::Rule,
        ev: event::Rule<W>,
    ) -> io::Result<()> {
        use event::Rule;

        match ev {
            Rule::Started => {
                self.rule_started(rule)?;
            }
            Rule::Scenario(sc, ev) => {
                self.scenario(feat, &sc, &ev)?;
            }
            Rule::Finished => {
                self.indent = self.indent.saturating_sub(2);
            }
        }
        Ok(())
    }

    /// Outputs the [started] [`Rule`].
    ///
    /// [started]: event::Rule::Started
    /// [`Rule`]: gherkin::Rule
    pub(crate) fn rule_started(
        &mut self,
        rule: &gherkin::Rule,
    ) -> io::Result<()> {
        let out = format!(
            "{indent}{}: {}",
            rule.keyword,
            rule.name,
            indent = " ".repeat(self.indent)
        );
        self.indent += 2;
        self.output.write_line(&self.styles.ok(out))
    }

    /// Outputs the [`Scenario`]'s [started]/[background]/[step] event.
    ///
    /// [background]: event::Scenario::Background
    /// [started]: event::Scenario::Started
    /// [step]: event::Step
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn scenario<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        scenario: &gherkin::Scenario,
        ev: &event::RetryableScenario<W>,
    ) -> io::Result<()> {
        use event::{Hook, Scenario};

        let retries = ev.retries;
        match &ev.event {
            Scenario::Started => {
                self.scenario_started(scenario, retries)?;
            }
            Scenario::Hook(_, Hook::Started) => {
                self.indent += 4;
            }
            Scenario::Hook(which, Hook::Failed(world, info)) => {
                self.hook_failed(
                    feat,
                    scenario,
                    *which,
                    retries,
                    world.as_ref(),
                    info,
                )?;
                self.indent = self.indent.saturating_sub(4);
            }
            Scenario::Hook(_, Hook::Passed) => {
                self.indent = self.indent.saturating_sub(4);
            }
            Scenario::Background(bg, ev) => {
                self.background(feat, scenario, bg, ev, retries)?;
            }
            Scenario::Step(st, ev) => {
                self.step(feat, scenario, st, ev, retries)?;
            }
            Scenario::Finished => {
                self.indent = self.indent.saturating_sub(2);
            }
            Scenario::Log(msg) => self.emit_log(msg)?,
        }
        Ok(())
    }

    /// Outputs the [`event::Scenario::Log`].
    pub(crate) fn emit_log(&mut self, msg: impl AsRef<str>) -> io::Result<()> {
        self.lines_to_clear += self.styles.lines_count(msg.as_ref());
        self.re_output_after_clear.push_str(msg.as_ref());
        self.output.write_str(msg)
    }

    /// Outputs the [failed] [`Scenario`]'s hook.
    ///
    /// [failed]: event::Hook::Failed
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn hook_failed<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        sc: &gherkin::Scenario,
        which: event::HookType,
        retries: Option<Retries>,
        world: Option<&W>,
        info: &Info,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let style = |s| {
            if retries.filter(|r| r.left > 0).is_some() {
                self.styles.bright().retry(s)
            } else {
                self.styles.err(s)
            }
        };

        self.output.write_line(&style(format!(
            "{indent}✘  Scenario's {which} hook failed {}:{}:{}\n\
             {indent}   Captured output: {}{}",
            feat.path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .unwrap_or(&feat.name),
            sc.position.line,
            sc.position.col,
            format_str_with_indent(
                coerce_error(info),
                self.indent.saturating_sub(3) + 3
            ),
            world
                .map(|w| format_str_with_indent(
                    format!("{w:#?}"),
                    self.indent.saturating_sub(3) + 3,
                ))
                .filter(|_| self.verbosity.shows_world())
                .unwrap_or_default(),
            indent = " ".repeat(self.indent.saturating_sub(3)),
        )))
    }

    /// Outputs the [started] [`Scenario`].
    ///
    /// [started]: event::Scenario::Started
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn scenario_started(
        &mut self,
        scenario: &gherkin::Scenario,
        retries: Option<Retries>,
    ) -> io::Result<()> {
        self.indent += 2;

        if let Some(retries) = retries.filter(|r| r.current > 0) {
            let out = format!(
                "{}{}: {} | Retry attempt: {}/{}",
                " ".repeat(self.indent),
                scenario.keyword,
                scenario.name,
                retries.current,
                retries.left + retries.current,
            );
            self.output.write_line(&self.styles.retry(out))
        } else {
            let out = format!(
                "{}{}: {}",
                " ".repeat(self.indent),
                scenario.keyword,
                scenario.name,
            );
            self.output.write_line(&self.styles.ok(out))
        }
    }

    /// Outputs the [`Step`]'s [started]/[passed]/[skipped]/[failed] event.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Step`]: gherkin::Step
    pub(crate) fn step<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        sc: &gherkin::Scenario,
        step: &gherkin::Step,
        ev: &event::Step<W>,
        retries: Option<Retries>,
    ) -> io::Result<()> {
        use event::Step;

        match ev {
            Step::Started => {
                self.step_started(step)?;
            }
            Step::Passed(captures, _) => {
                self.step_passed(sc, step, captures, retries)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Skipped => {
                self.step_skipped(feat, step)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Failed(c, loc, w, i) => {
                self.step_failed(
                    feat,
                    step,
                    c.as_ref(),
                    *loc,
                    retries,
                    w.as_ref(),
                    i,
                )?;
                self.indent = self.indent.saturating_sub(4);
            }
        }
        Ok(())
    }

    /// Outputs the [started] [`Step`].
    ///
    /// The [`Step`] is printed only if [`Coloring`] is enabled and gets
    /// overwritten by later [passed]/[skipped]/[failed] events.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Step`]: gherkin::Step
    pub(crate) fn step_started(
        &mut self,
        step: &gherkin::Step,
    ) -> io::Result<()> {
        self.indent += 4;
        if self.styles.is_present {
            let out = format!(
                "{indent}{}{}{}{}",
                step.keyword,
                step.value,
                step.docstring
                    .as_ref()
                    .and_then(|doc| self.verbosity.shows_docstring().then(
                        || {
                            format_str_with_indent(
                                doc,
                                self.indent.saturating_sub(3) + 3,
                            )
                        }
                    ))
                    .unwrap_or_default(),
                step.table
                    .as_ref()
                    .map(|t| format_table(t, self.indent))
                    .unwrap_or_default(),
                indent = " ".repeat(self.indent),
            );
            self.lines_to_clear += self.styles.lines_count(&out);
            self.output.write_line(&out)?;
        }
        Ok(())
    }

    /// Outputs the [passed] [`Step`].
    ///
    /// [passed]: event::Step::Passed
    /// [`Step`]: gherkin::Step
    pub(crate) fn step_passed(
        &mut self,
        scenario: &gherkin::Scenario,
        step: &gherkin::Step,
        captures: &CaptureLocations,
        retries: Option<Retries>,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let style = |s| {
            if retries.filter(|r| r.current > 0).is_some()
                && scenario.steps.last().filter(|st| *st != step).is_some()
            {
                self.styles.retry(s)
            } else {
                self.styles.ok(s)
            }
        };

        let step_keyword = style(format!("✔  {}", step.keyword));
        let step_value = format_captures(
            &step.value,
            captures,
            |v| style(v.to_owned()),
            |v| style(self.styles.bold(v).to_string()),
        );
        let doc_str = style(
            step.docstring
                .as_ref()
                .and_then(|doc| {
                    self.verbosity.shows_docstring().then(|| {
                        format_str_with_indent(
                            doc,
                            self.indent.saturating_sub(3) + 3,
                        )
                    })
                })
                .unwrap_or_default(),
        );
        let step_table = style(
            step.table
                .as_ref()
                .map(|t| format_table(t, self.indent))
                .unwrap_or_default(),
        );

        self.output.write_line(&style(format!(
            "{indent}{step_keyword}{step_value}{doc_str}{step_table}",
            indent = " ".repeat(self.indent.saturating_sub(3)),
        )))
    }

    /// Outputs the [skipped] [`Step`].
    ///
    /// [skipped]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    pub(crate) fn step_skipped(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;
        self.output.write_line(&self.styles.skipped(format!(
            "{indent}?  {}{}{}{}\n\
             {indent}   Step skipped: {}:{}:{}",
            step.keyword,
            step.value,
            step.docstring
                .as_ref()
                .and_then(|doc| self.verbosity.shows_docstring().then(|| {
                    format_str_with_indent(
                        doc,
                        self.indent.saturating_sub(3) + 3,
                    )
                }))
                .unwrap_or_default(),
            step.table
                .as_ref()
                .map(|t| format_table(t, self.indent))
                .unwrap_or_default(),
            feat.path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            indent = " ".repeat(self.indent.saturating_sub(3)),
        )))
    }

    /// Outputs the [failed] [`Step`].
    ///
    /// [failed]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step_failed<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        captures: Option<&CaptureLocations>,
        loc: Option<step::Location>,
        retries: Option<Retries>,
        world: Option<&W>,
        err: &event::StepError,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let style = |s| {
            if retries
                .filter(|r| {
                    r.left > 0 && !matches!(err, event::StepError::NotFound)
                })
                .is_some()
            {
                self.styles.bright().retry(s)
            } else {
                self.styles.err(s)
            }
        };

        let indent = " ".repeat(self.indent.saturating_sub(3));

        let step_keyword = style(format!("{indent}✘  {}", step.keyword));
        let step_value = captures.map_or_else(
            || style(step.value.clone()),
            |capts| {
                format_captures(
                    &step.value,
                    capts,
                    |v| style(v.to_owned()),
                    |v| style(self.styles.bold(v).to_string()),
                )
                .into()
            },
        );

        let diagnostics = style(format!(
            "{}{}\n\
             {indent}   Step failed:\n\
             {indent}   Defined: {}:{}:{}{}{}{}",
            step.docstring
                .as_ref()
                .and_then(|doc| self.verbosity.shows_docstring().then(|| {
                    format_str_with_indent(
                        doc,
                        self.indent.saturating_sub(3) + 3,
                    )
                }))
                .unwrap_or_default(),
            step.table
                .as_ref()
                .map(|t| format_table(t, self.indent))
                .unwrap_or_default(),
            feat.path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            loc.map(|l| format!(
                "\n{indent}   Matched: {}:{}:{}",
                l.path, l.line, l.column,
            ))
            .unwrap_or_default(),
            format_str_with_indent(
                err.to_string(),
                self.indent.saturating_sub(3) + 3,
            ),
            world
                .map(|w| format_str_with_indent(
                    format!("{w:#?}"),
                    self.indent.saturating_sub(3) + 3,
                ))
                .filter(|_| self.verbosity.shows_world())
                .unwrap_or_default(),
        ));

        self.output
            .write_line(&format!("{step_keyword}{step_value}{diagnostics}"))
    }

    /// Outputs the [`Background`] [`Step`]'s
    /// [started]/[passed]/[skipped]/[failed] event.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub(crate) fn background<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        sc: &gherkin::Scenario,
        bg: &gherkin::Step,
        ev: &event::Step<W>,
        retries: Option<Retries>,
    ) -> io::Result<()> {
        use event::Step;

        match ev {
            Step::Started => {
                self.bg_step_started(bg)?;
            }
            Step::Passed(captures, _) => {
                self.bg_step_passed(sc, bg, captures, retries)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Skipped => {
                self.bg_step_skipped(feat, bg)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Failed(c, loc, w, i) => {
                self.bg_step_failed(
                    feat,
                    bg,
                    c.as_ref(),
                    *loc,
                    retries,
                    w.as_ref(),
                    i,
                )?;
                self.indent = self.indent.saturating_sub(4);
            }
        }
        Ok(())
    }

    /// Outputs the [started] [`Background`] [`Step`].
    ///
    /// The [`Step`] is printed only if [`Coloring`] is enabled and gets
    /// overwritten by later [passed]/[skipped]/[failed] events.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub(crate) fn bg_step_started(
        &mut self,
        step: &gherkin::Step,
    ) -> io::Result<()> {
        self.indent += 4;
        if self.styles.is_present {
            let out = format!(
                "{indent}> {}{}{}{}",
                step.keyword,
                step.value,
                step.docstring
                    .as_ref()
                    .and_then(|doc| self.verbosity.shows_docstring().then(
                        || {
                            format_str_with_indent(
                                doc,
                                self.indent.saturating_sub(3) + 3,
                            )
                        }
                    ))
                    .unwrap_or_default(),
                step.table
                    .as_ref()
                    .map(|t| format_table(t, self.indent))
                    .unwrap_or_default(),
                indent = " ".repeat(self.indent.saturating_sub(2)),
            );
            self.lines_to_clear += self.styles.lines_count(&out);
            self.output.write_line(&out)?;
        }
        Ok(())
    }

    /// Outputs the [passed] [`Background`] [`Step`].
    ///
    /// [passed]: event::Step::Passed
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub(crate) fn bg_step_passed(
        &mut self,
        scenario: &gherkin::Scenario,
        step: &gherkin::Step,
        captures: &CaptureLocations,
        retries: Option<Retries>,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let style = |s| {
            if retries.filter(|r| r.current > 0).is_some()
                && scenario.steps.last().filter(|st| *st != step).is_some()
            {
                self.styles.retry(s)
            } else {
                self.styles.ok(s)
            }
        };

        let indent = " ".repeat(self.indent.saturating_sub(3));

        let step_keyword = style(format!("{indent}✔> {}", step.keyword));
        let step_value = format_captures(
            &step.value,
            captures,
            |v| style(v.to_owned()),
            |v| style(self.styles.bold(v).to_string()),
        );
        let doc_str = style(
            step.docstring
                .as_ref()
                .and_then(|doc| {
                    self.verbosity.shows_docstring().then(|| {
                        format_str_with_indent(
                            doc,
                            self.indent.saturating_sub(3) + 3,
                        )
                    })
                })
                .unwrap_or_default(),
        );
        let step_table = style(
            step.table
                .as_ref()
                .map(|t| format_table(t, self.indent))
                .unwrap_or_default(),
        );

        self.output.write_line(&style(format!(
            "{step_keyword}{step_value}{doc_str}{step_table}",
        )))
    }

    /// Outputs the [skipped] [`Background`] [`Step`].
    ///
    /// [skipped]: event::Step::Skipped
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub(crate) fn bg_step_skipped(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;
        self.output.write_line(&self.styles.skipped(format!(
            "{indent}?> {}{}{}{}\n\
             {indent}   Background step failed: {}:{}:{}",
            step.keyword,
            step.value,
            step.docstring
                .as_ref()
                .and_then(|doc| self.verbosity.shows_docstring().then(|| {
                    format_str_with_indent(
                        doc,
                        self.indent.saturating_sub(3) + 3,
                    )
                }))
                .unwrap_or_default(),
            step.table
                .as_ref()
                .map(|t| format_table(t, self.indent))
                .unwrap_or_default(),
            feat.path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            indent = " ".repeat(self.indent.saturating_sub(3)),
        )))
    }

    /// Outputs the [failed] [`Background`] [`Step`].
    ///
    /// [failed]: event::Step::Failed
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn bg_step_failed<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        captures: Option<&CaptureLocations>,
        loc: Option<step::Location>,
        retries: Option<Retries>,
        world: Option<&W>,
        err: &event::StepError,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let style = |s| {
            if retries
                .filter(|r| {
                    r.left > 0 && !matches!(err, event::StepError::NotFound)
                })
                .is_some()
            {
                self.styles.bright().retry(s)
            } else {
                self.styles.err(s)
            }
        };

        let indent = " ".repeat(self.indent.saturating_sub(3));
        let step_keyword = style(format!("{indent}✘> {}", step.keyword));
        let step_value = captures.map_or_else(
            || style(step.value.clone()),
            |capts| {
                format_captures(
                    &step.value,
                    capts,
                    |v| style(v.to_owned()),
                    |v| style(self.styles.bold(v).to_string()),
                )
                .into()
            },
        );

        let diagnostics = style(format!(
            "{}{}\n\
             {indent}   Step failed:\n\
             {indent}   Defined: {}:{}:{}{}{}{}",
            step.docstring
                .as_ref()
                .and_then(|doc| self.verbosity.shows_docstring().then(|| {
                    format_str_with_indent(
                        doc,
                        self.indent.saturating_sub(3) + 3,
                    )
                }))
                .unwrap_or_default(),
            step.table
                .as_ref()
                .map(|t| format_table(t, self.indent))
                .unwrap_or_default(),
            feat.path
                .as_ref()
                .and_then(|p| p.to_str().map(trim_path))
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            loc.map(|l| format!(
                "\n{indent}   Matched: {}:{}:{}",
                l.path, l.line, l.column,
            ))
            .unwrap_or_default(),
            format_str_with_indent(
                err.to_string(),
                self.indent.saturating_sub(3) + 3,
            ),
            world
                .map(|w| format_str_with_indent(
                    format!("{w:#?}"),
                    self.indent.saturating_sub(3) + 3,
                ))
                .filter(|_| self.verbosity.shows_world())
                .unwrap_or_default(),
        ));

        self.output
            .write_line(&format!("{step_keyword}{step_value}{diagnostics}"))
    }
}

/// Tries to coerce [`catch_unwind()`] output to [`String`].
///
/// [`catch_unwind()`]: std::panic::catch_unwind()
#[must_use]
pub(crate) fn coerce_error(err: &Info) -> Cow<'static, str> {
    err.downcast_ref::<String>()
        .map(|s| s.clone().into())
        .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_owned().into()))
        .unwrap_or_else(|| "(Could not resolve panic payload)".into())
}

/// Formats the given [`str`] by adding `indent`s to each line to prettify the
/// output.
fn format_str_with_indent(str: impl AsRef<str>, indent: usize) -> String {
    let str = str
        .as_ref()
        .lines()
        .map(|line| format!("{}{line}", " ".repeat(indent)))
        .join("\n");
    (!str.is_empty())
        .then(|| format!("\n{str}"))
        .unwrap_or_default()
}

/// Formats the given [`gherkin::Table`] and adds `indent`s to each line to
/// prettify the output.
fn format_table(table: &gherkin::Table, indent: usize) -> String {
    use std::fmt::Write as _;

    let max_row_len = table
        .rows
        .iter()
        .fold(None, |mut acc: Option<Vec<_>>, row| {
            // false positive: due to mut borrowing
            #[allow(clippy::option_if_let_else)]
            if let Some(existing_len) = acc.as_mut() {
                for (cell, max_len) in row.iter().zip(existing_len) {
                    *max_len = cmp::max(*max_len, cell.len());
                }
            } else {
                acc = Some(row.iter().map(String::len).collect::<Vec<_>>());
            }
            acc
        })
        .unwrap_or_default();

    let mut table = table
        .rows
        .iter()
        .map(|row| {
            row.iter().zip(&max_row_len).fold(
                String::new(),
                |mut out, (cell, len)| {
                    _ = write!(out, "| {cell:len$} ");
                    out
                },
            )
        })
        .map(|row| format!("{}{row}", " ".repeat(indent + 1)))
        .join("|\n");

    if !table.is_empty() {
        table.insert(0, '\n');
        table.push('|');
    }

    table
}

/// Formats `value`s in the given `captures` with the provided `accent` style
/// and with the `default` style anything else.
fn format_captures<D, A>(
    value: impl AsRef<str>,
    captures: &CaptureLocations,
    default: D,
    accent: A,
) -> String
where
    D: for<'a> Fn(&'a str) -> Cow<'a, str>,
    A: for<'a> Fn(&'a str) -> Cow<'a, str>,
{
    // PANIC: Slicing is OK here, as all indices are obtained from the source
    //        string.
    #![allow(clippy::string_slice)]

    let value = value.as_ref();

    let (mut formatted, end) = (1..captures.len())
        .filter_map(|group| captures.get(group))
        .fold(
            (String::with_capacity(value.len()), 0),
            |(mut str, old), (start, end)| {
                // Ignore nested groups.
                if old > start {
                    return (str, old);
                }

                str.push_str(&default(&value[old..start]));
                str.push_str(&accent(&value[start..end]));
                (str, end)
            },
        );
    formatted.push_str(&default(&value[end..value.len()]));

    formatted
}

/// Trims start of the path if it matches the current project directory.
pub(crate) fn trim_path(path: &str) -> &str {
    /// Path of the current project directory.
    static CURRENT_DIR: Lazy<String> = Lazy::new(|| {
        env::var("CARGO_WORKSPACE_DIR")
            .or_else(|_| env::var("CARGO_MANIFEST_DIR"))
            .unwrap_or_else(|_| {
                env::current_dir()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
            })
    });

    path.trim_start_matches(&**CURRENT_DIR)
        .trim_start_matches('/')
        .trim_start_matches('\\')
}
