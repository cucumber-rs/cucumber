// Copyright (c) 2018-2022  Brendan Molloy <brendan@bbqsrc.net>,
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
    cmp,
    fmt::{Debug, Display},
    io,
    str::FromStr,
};

use async_trait::async_trait;
use derive_more::{Deref, DerefMut};
use itertools::Itertools as _;
use regex::CaptureLocations;

use crate::{
    cli::Colored,
    event::{self, Info},
    parser,
    writer::{
        self,
        out::{Styles, WriteStrExt as _},
        Ext as _, Verbosity,
    },
    Event, World, Writer,
};

/// CLI options of a [`Basic`] [`Writer`].
#[derive(clap::Args, Clone, Copy, Debug)]
pub struct Cli {
    /// Verbosity of an output.
    ///
    /// `-v` is default verbosity, `-vv` additionally outputs world on failed
    /// steps, `-vvv` additionally outputs step's doc string (if present).
    #[clap(short, parse(from_occurrences))]
    pub verbose: u8,

    /// Coloring policy for a console output.
    #[clap(long, name = "auto|always|never", default_value = "auto")]
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
#[derive(Debug, Deref, DerefMut)]
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

    #[allow(clippy::unused_async)] // false positive: #[async_trait]
    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        opts: &Self::Cli,
    ) {
        use event::{Cucumber, Feature};

        self.apply_cli(*opts);

        match ev.map(Event::into_inner) {
            Err(err) => self.parsing_failed(&err),
            Ok(Cucumber::Started | Cucumber::Finished) => Ok(()),
            Ok(Cucumber::Feature(f, ev)) => match ev {
                Feature::Started => self.feature_started(&f),
                Feature::Scenario(sc, ev) => self.scenario(&f, &sc, &ev),
                Feature::Rule(r, ev) => self.rule(&f, &r, ev),
                Feature::Finished => Ok(()),
            },
        }
        .unwrap_or_else(|e| panic!("Failed to write into terminal: {}", e));
    }
}

#[async_trait(?Send)]
impl<'val, W, Val, Out> writer::Arbitrary<'val, W, Val> for Basic<Out>
where
    W: World + Debug,
    Val: AsRef<str> + 'val,
    Out: io::Write,
{
    #[allow(clippy::unused_async)] // false positive: #[async_trait]
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.write_line(val.as_ref())
            .unwrap_or_else(|e| panic!("Failed to write: {}", e));
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
            .write_line(&self.styles.err(format!("Failed to parse: {}", error)))
    }

    /// Outputs the [started] [`Feature`].
    ///
    /// [started]: event::Feature::Started
    /// [`Feature`]: gherkin::Feature
    pub(crate) fn feature_started(
        &mut self,
        feature: &gherkin::Feature,
    ) -> io::Result<()> {
        self.lines_to_clear = 1;
        self.output.write_line(
            &self
                .styles
                .ok(format!("{}: {}", feature.keyword, feature.name)),
        )
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
        self.lines_to_clear = 1;
        self.indent += 2;
        self.output.write_line(&self.styles.ok(format!(
            "{indent}{}: {}",
            rule.keyword,
            rule.name,
            indent = " ".repeat(self.indent)
        )))
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
        ev: &event::Scenario<W>,
    ) -> io::Result<()> {
        use event::{Hook, Scenario};

        match ev {
            Scenario::Started => {
                self.scenario_started(scenario)?;
            }
            Scenario::Hook(_, Hook::Started) => {
                self.indent += 4;
            }
            Scenario::Hook(which, Hook::Failed(world, info)) => {
                self.hook_failed(feat, scenario, *which, world.as_ref(), info)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Scenario::Hook(_, Hook::Passed) => {
                self.indent = self.indent.saturating_sub(4);
            }
            Scenario::Background(bg, ev) => {
                self.background(feat, bg, ev)?;
            }
            Scenario::Step(st, ev) => {
                self.step(feat, st, ev)?;
            }
            Scenario::Finished => self.indent = self.indent.saturating_sub(2),
        }
        Ok(())
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
        world: Option<&W>,
        info: &Info,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        self.output.write_line(&self.styles.err(format!(
            "{indent}\u{2718}  Scenario's {} hook failed {}:{}:{}\n\
             {indent}   Captured output: {}{}",
            which,
            feat.path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(&feat.name),
            sc.position.line,
            sc.position.col,
            coerce_error(info),
            world
                .map(|w| format_str_with_indent(
                    format!("{:#?}", w),
                    self.indent.saturating_sub(3) + 3,
                ))
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
    ) -> io::Result<()> {
        self.lines_to_clear = 1;
        self.indent += 2;
        self.output.write_line(&self.styles.ok(format!(
            "{}{}: {}",
            " ".repeat(self.indent),
            scenario.keyword,
            scenario.name,
        )))
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
        step: &gherkin::Step,
        ev: &event::Step<W>,
    ) -> io::Result<()> {
        use event::Step;

        match ev {
            Step::Started => {
                self.step_started(step)?;
            }
            Step::Passed(captures) => {
                self.step_passed(step, captures)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Skipped => {
                self.step_skipped(feat, step)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Failed(c, w, i) => {
                self.step_failed(feat, step, c.as_ref(), w.as_ref(), i)?;
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
            let output = format!(
                "{indent}{} {}{}{}",
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
            self.lines_to_clear = output.lines().count();
            self.write_line(&output)?;
        }
        Ok(())
    }

    /// Outputs the [passed] [`Step`].
    ///
    /// [passed]: event::Step::Passed
    /// [`Step`]: gherkin::Step
    pub(crate) fn step_passed(
        &mut self,
        step: &gherkin::Step,
        captures: &CaptureLocations,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let step_keyword =
            self.styles.ok(format!("\u{2714}  {}", step.keyword));
        let step_value = format_captures(
            &step.value,
            captures,
            |v| self.styles.ok(v),
            |v| self.styles.ok(self.styles.bold(v)),
        );
        let doc_str = self.styles.ok(step
            .docstring
            .as_ref()
            .and_then(|doc| {
                self.verbosity.shows_docstring().then(|| {
                    format_str_with_indent(
                        doc,
                        self.indent.saturating_sub(3) + 3,
                    )
                })
            })
            .unwrap_or_default());
        let step_table = self.styles.ok(step
            .table
            .as_ref()
            .map(|t| format_table(t, self.indent))
            .unwrap_or_default());

        self.output.write_line(&self.styles.ok(format!(
            "{indent}{} {}{}{}",
            step_keyword,
            step_value,
            doc_str,
            step_table,
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
            "{indent}?  {} {}{}{}\n\
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
                .and_then(|p| p.to_str())
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
    pub(crate) fn step_failed<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        captures: Option<&CaptureLocations>,
        world: Option<&W>,
        err: &event::StepError,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let step_keyword = self.styles.err(format!(
            "{indent}\u{2718}  {}",
            step.keyword,
            indent = " ".repeat(self.indent.saturating_sub(3)),
        ));
        let step_value = captures.map_or_else(
            || self.styles.err(&step.value),
            |capts| {
                format_captures(
                    &step.value,
                    capts,
                    |v| self.styles.err(v),
                    |v| self.styles.err(self.styles.bold(v)),
                )
                .into()
            },
        );

        let diagnostics = self.styles.err(format!(
            "{}{}\n\
             {indent}   Step failed: {}:{}:{}\n\
             {indent}   Captured output: {}{}",
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
                .and_then(|p| p.to_str())
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            format_str_with_indent(
                format!("{}", err),
                self.indent.saturating_sub(3) + 3,
            ),
            world
                .map(|w| format_str_with_indent(
                    format!("{:#?}", w),
                    self.indent.saturating_sub(3) + 3,
                ))
                .filter(|_| self.verbosity.shows_world())
                .unwrap_or_default(),
            indent = " ".repeat(self.indent.saturating_sub(3))
        ));

        self.write_line(&format!(
            "{} {}{}",
            step_keyword, step_value, diagnostics,
        ))
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
        bg: &gherkin::Step,
        ev: &event::Step<W>,
    ) -> io::Result<()> {
        use event::Step;

        match ev {
            Step::Started => {
                self.bg_step_started(bg)?;
            }
            Step::Passed(captures) => {
                self.bg_step_passed(bg, captures)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Skipped => {
                self.bg_step_skipped(feat, bg)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Failed(c, w, i) => {
                self.bg_step_failed(feat, bg, c.as_ref(), w.as_ref(), i)?;
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
            let output = format!(
                "{indent}> {} {}{}{}",
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
            self.lines_to_clear = output.lines().count();
            self.write_line(&output)?;
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
        step: &gherkin::Step,
        captures: &CaptureLocations,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let step_keyword =
            self.styles.ok(format!("\u{2714}> {}", step.keyword));
        let step_value = format_captures(
            &step.value,
            captures,
            |v| self.styles.ok(v),
            |v| self.styles.ok(self.styles.bold(v)),
        );
        let doc_str = self.styles.ok(step
            .docstring
            .as_ref()
            .and_then(|doc| {
                self.verbosity.shows_docstring().then(|| {
                    format_str_with_indent(
                        doc,
                        self.indent.saturating_sub(3) + 3,
                    )
                })
            })
            .unwrap_or_default());
        let step_table = self.styles.ok(step
            .table
            .as_ref()
            .map(|t| format_table(t, self.indent))
            .unwrap_or_default());

        self.output.write_line(&self.styles.ok(format!(
            "{indent}{} {}{}{}",
            step_keyword,
            step_value,
            doc_str,
            step_table,
            indent = " ".repeat(self.indent.saturating_sub(3)),
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
            "{indent}?> {} {}{}{}\n\
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
                .and_then(|p| p.to_str())
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
    pub(crate) fn bg_step_failed<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        captures: Option<&CaptureLocations>,
        world: Option<&W>,
        err: &event::StepError,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let step_keyword = self.styles.err(format!(
            "{indent}\u{2718}> {}{}",
            step.keyword,
            indent = " ".repeat(self.indent.saturating_sub(3)),
        ));
        let step_value = captures.map_or_else(
            || self.styles.err(&step.value),
            |capts| {
                format_captures(
                    &step.value,
                    capts,
                    |v| self.styles.err(v),
                    |v| self.styles.err(self.styles.bold(v)),
                )
                .into()
            },
        );

        let diagnostics = self.styles.err(format!(
            "{}{}\n\
             {indent}   Step failed: {}:{}:{}\n\
             {indent}   Captured output: {}{}",
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
                .and_then(|p| p.to_str())
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            format_str_with_indent(
                format!("{}", err),
                self.indent.saturating_sub(3) + 3,
            ),
            world
                .map(|w| format_str_with_indent(
                    format!("{:#?}", w),
                    self.indent.saturating_sub(3) + 3,
                ))
                .unwrap_or_default(),
            indent = " ".repeat(self.indent.saturating_sub(3))
        ));

        self.write_line(&format!(
            "{} {}{}",
            step_keyword, step_value, diagnostics,
        ))
    }
}

/// Tries to coerce [`catch_unwind()`] output to [`String`].
///
/// [`catch_unwind()`]: std::panic::catch_unwind()
#[must_use]
pub(crate) fn coerce_error(err: &Info) -> Cow<'static, str> {
    if let Some(string) = err.downcast_ref::<String>() {
        string.clone().into()
    } else if let Some(&string) = err.downcast_ref::<&str>() {
        string.to_owned().into()
    } else {
        "(Could not resolve panic payload)".into()
    }
}

/// Formats the given [`str`] by adding `indent`s to each line to prettify the
/// output.
fn format_str_with_indent(str: impl AsRef<str>, indent: usize) -> String {
    let str = str
        .as_ref()
        .lines()
        .map(|line| format!("{}{}", " ".repeat(indent), line))
        .join("\n");
    (!str.is_empty())
        .then(|| format!("\n{}", str))
        .unwrap_or_default()
}

/// Formats the given [`gherkin::Table`] and adds `indent`s to each line to
/// prettify the output.
fn format_table(table: &gherkin::Table, indent: usize) -> String {
    let max_row_len = table
        .rows
        .iter()
        .fold(None, |mut acc: Option<Vec<_>>, row| {
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
            row.iter()
                .zip(&max_row_len)
                .map(|(cell, len)| format!("| {:1$} ", cell, len))
                .collect::<String>()
        })
        .map(|row| format!("{}{}", " ".repeat(indent + 1), row))
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
