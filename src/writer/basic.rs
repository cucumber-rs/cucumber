// Copyright (c) 2018-2021  Brendan Molloy <brendan@bbqsrc.net>,
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
    ops::Deref,
};

use async_trait::async_trait;
use console::Term;
use itertools::Itertools as _;
use regex::CaptureLocations;

use crate::{
    event::{self, Info},
    parser,
    writer::term::Styles,
    ArbitraryWriter, World, Writer,
};

/// Default [`Writer`] implementation outputting to [`Term`]inal (STDOUT by
/// default).
///
/// Pretty-prints with colors if terminal was successfully detected, otherwise
/// has simple output. Useful for running tests with CI tools.
#[derive(Debug)]
pub struct Basic {
    /// Terminal to write the output into.
    terminal: Term,

    /// [`Styles`] for terminal output.
    styles: Styles,

    /// Current indentation that events are outputted with.
    indent: usize,

    /// Number of lines to clear.
    lines_to_clear: usize,
}

#[async_trait(?Send)]
impl<W: World + Debug> Writer<W> for Basic {
    #[allow(clippy::unused_async)] // false positive: #[async_trait]
    async fn handle_event(&mut self, ev: parser::Result<event::Cucumber<W>>) {
        use event::{Cucumber, Feature};

        match ev {
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
impl<'val, W, Val> ArbitraryWriter<'val, W, Val> for Basic
where
    W: World + Debug,
    Val: AsRef<str> + 'val,
{
    #[allow(clippy::unused_async)] // false positive: #[async_trait]
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.write_line(val.as_ref())
            .unwrap_or_else(|e| panic!("Failed to write into terminal: {}", e));
    }
}

impl Default for Basic {
    fn default() -> Self {
        Self {
            terminal: Term::stdout(),
            styles: Styles::new(),
            indent: 0,
            lines_to_clear: 0,
        }
    }
}

impl Deref for Basic {
    type Target = Term;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl Basic {
    /// Creates a new [`Basic`] [`Writer`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears last `n` lines if terminal is present.
    fn clear_last_lines_if_term_present(&mut self) -> io::Result<()> {
        if self.styles.is_present && self.lines_to_clear > 0 {
            self.clear_last_lines(self.lines_to_clear)?;
            self.lines_to_clear = 0;
        }
        Ok(())
    }

    /// Outputs the parsing `error` encountered while parsing some [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    fn parsing_failed(&self, error: impl Display) -> io::Result<()> {
        self.write_line(&self.styles.err(format!("Failed to parse: {}", error)))
    }

    /// Outputs [started] [`Feature`] to STDOUT.
    ///
    /// [started]: event::Feature::Started
    /// [`Feature`]: gherkin::Feature
    fn feature_started(
        &mut self,
        feature: &gherkin::Feature,
    ) -> io::Result<()> {
        self.lines_to_clear = 1;
        self.write_line(
            &self
                .styles
                .ok(format!("{}: {}", feature.keyword, feature.name)),
        )
    }

    /// Outputs [`Rule`] [started]/[scenario]/[finished] event to STDOUT.
    ///
    /// [finished]: event::Rule::Finished
    /// [scenario]: event::Rule::Scenario
    /// [started]: event::Rule::Started
    /// [`Rule`]: gherkin::Rule
    fn rule<W: Debug>(
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

    /// Outputs [started] [`Rule`] to STDOUT.
    ///
    /// [started]: event::Rule::Started
    /// [`Rule`]: gherkin::Rule
    fn rule_started(&mut self, rule: &gherkin::Rule) -> io::Result<()> {
        self.lines_to_clear = 1;
        self.indent += 2;
        self.write_line(&self.styles.ok(format!(
            "{indent}{}: {}",
            rule.keyword,
            rule.name,
            indent = " ".repeat(self.indent)
        )))
    }

    /// Outputs [`Scenario`] [started]/[background]/[step] event to STDOUT.
    ///
    /// [background]: event::Scenario::Background
    /// [started]: event::Scenario::Started
    /// [step]: event::Step
    /// [`Scenario`]: gherkin::Scenario
    fn scenario<W: Debug>(
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

    /// Outputs [failed] [`Scenario`]'s hook to STDOUT.
    ///
    /// [failed]: event::Hook::Failed
    /// [`Scenario`]: gherkin::Scenario
    fn hook_failed<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        sc: &gherkin::Scenario,
        which: event::HookType,
        world: Option<&W>,
        info: &Info,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        self.write_line(&self.styles.err(format!(
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
            format_debug_with_indent(world, self.indent.saturating_sub(3) + 3),
            indent = " ".repeat(self.indent.saturating_sub(3)),
        )))
    }

    /// Outputs [started] [`Scenario`] to STDOUT.
    ///
    /// [started]: event::Scenario::Started
    /// [`Scenario`]: gherkin::Scenario
    fn scenario_started(
        &mut self,
        scenario: &gherkin::Scenario,
    ) -> io::Result<()> {
        self.lines_to_clear = 1;
        self.indent += 2;
        self.write_line(&self.styles.ok(format!(
            "{}{}: {}",
            " ".repeat(self.indent),
            scenario.keyword,
            scenario.name,
        )))
    }

    /// Outputs [`Step`] [started]/[passed]/[skipped]/[failed] event to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Step`]: gherkin::Step
    fn step<W: Debug>(
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
            Step::Failed(caps, w, info) => {
                self.step_failed(feat, step, caps.as_ref(), w.as_ref(), info)?;
                self.indent = self.indent.saturating_sub(4);
            }
        }
        Ok(())
    }

    /// Outputs [started] [`Step`] to STDOUT.
    ///
    /// This [`Step`] is printed only if terminal is present and gets
    /// overwritten by later [passed]/[skipped]/[failed] events.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Step`]: gherkin::Step
    fn step_started(&mut self, step: &gherkin::Step) -> io::Result<()> {
        self.indent += 4;
        if self.styles.is_present {
            let output = format!(
                "{indent}{} {}{}",
                step.keyword,
                step.value,
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

    /// Outputs [passed] [`Step`] to STDOUT.
    ///
    /// [passed]: event::Step::Passed
    /// [`Step`]: gherkin::Step
    fn step_passed(
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
        let step_table = self.styles.ok(step
            .table
            .as_ref()
            .map(|t| format_table(t, self.indent))
            .unwrap_or_default());

        self.write_line(&self.styles.ok(format!(
            "{indent}{} {}{}",
            step_keyword,
            step_value,
            step_table,
            indent = " ".repeat(self.indent.saturating_sub(3)),
        )))
    }

    /// Outputs [skipped] [`Step`] to STDOUT.
    ///
    /// [skipped]: event::Step::Skipped
    /// [`Step`]: gherkin::Step
    fn step_skipped(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;
        self.write_line(&self.styles.skipped(format!(
            "{indent}?  {} {}{}\n\
             {indent}   Step skipped: {}:{}:{}",
            step.keyword,
            step.value,
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

    /// Outputs [failed] [`Step`] to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [`Step`]: gherkin::Step
    fn step_failed<W: Debug>(
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
            |captures| {
                format_captures(
                    &step.value,
                    captures,
                    |v| self.styles.err(v),
                    |v| self.styles.err(self.styles.bold(v)),
                )
                .into()
            },
        );

        let diagnostics = self.styles.err(format!(
            "{}\n\
             {indent}   Step failed: {}:{}:{}\n\
             {indent}   Captured output: {}{}",
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
            format_display_with_indent(err, self.indent.saturating_sub(3) + 3),
            format_debug_with_indent(world, self.indent.saturating_sub(3) + 3),
            indent = " ".repeat(self.indent.saturating_sub(3))
        ));

        self.write_line(&format!(
            "{} {}{}",
            step_keyword, step_value, diagnostics,
        ))
    }

    /// Outputs [`Background`] [`Step`] [started]/[passed]/[skipped]/[failed]
    /// event to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    fn background<W: Debug>(
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
            Step::Failed(caps, w, info) => {
                self.bg_step_failed(feat, bg, caps.as_ref(), w.as_ref(), info)?;
                self.indent = self.indent.saturating_sub(4);
            }
        }
        Ok(())
    }

    /// Outputs [started] [`Background`] [`Step`] to STDOUT.
    ///
    /// This [`Step`] is printed only if terminal is present and gets
    /// overwritten by later [passed]/[skipped]/[failed] events.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    fn bg_step_started(&mut self, step: &gherkin::Step) -> io::Result<()> {
        self.indent += 4;
        if self.styles.is_present {
            let output = format!(
                "{indent}> {} {}{}",
                step.keyword,
                step.value,
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

    /// Outputs [passed] [`Background`] [`Step`] to STDOUT.
    ///
    /// [passed]: event::Step::Passed
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    fn bg_step_passed(
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
        let step_table = self.styles.ok(step
            .table
            .as_ref()
            .map(|t| format_table(t, self.indent))
            .unwrap_or_default());

        self.write_line(&self.styles.ok(format!(
            "{indent}{} {}{}",
            step_keyword,
            step_value,
            step_table,
            indent = " ".repeat(self.indent.saturating_sub(3)),
        )))
    }

    /// Outputs [skipped] [`Background`] [`Step`] to STDOUT.
    ///
    /// [skipped]: event::Step::Skipped
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    fn bg_step_skipped(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;
        self.write_line(&self.styles.skipped(format!(
            "{indent}?> {} {}{}\n\
             {indent}   Background step failed: {}:{}:{}",
            step.keyword,
            step.value,
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

    /// Outputs [failed] [`Background`] [`Step`] to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    fn bg_step_failed<W: Debug>(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        captures: Option<&CaptureLocations>,
        world: Option<&W>,
        err: &event::StepError,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;

        let step_keyword = self.styles.err(format!(
            "{indent}\u{2718}> {}",
            step.keyword,
            indent = " ".repeat(self.indent.saturating_sub(3)),
        ));
        let step_value = captures.map_or_else(
            || self.styles.err(&step.value),
            |captures| {
                format_captures(
                    &step.value,
                    captures,
                    |v| self.styles.err(v),
                    |v| self.styles.err(self.styles.bold(v)),
                )
                .into()
            },
        );

        let diagnostics = self.styles.err(format!(
            "{}\n\
             {indent}   Step failed: {}:{}:{}\n\
             {indent}   Captured output: {}{}",
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
            format_display_with_indent(err, self.indent.saturating_sub(3) + 3),
            format_debug_with_indent(world, self.indent.saturating_sub(3) + 3),
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

/// Formats the given [`Debug`] implementor, then adds `indent`s to each line to
/// prettify the output.
fn format_debug_with_indent<'d, D, I>(debug: I, indent: usize) -> String
where
    D: Debug + 'd,
    I: Into<Option<&'d D>>,
{
    let debug = debug
        .into()
        .map(|debug| format!("{:#?}", debug))
        .unwrap_or_default()
        .lines()
        .map(|line| format!("{}{}", " ".repeat(indent), line))
        .join("\n");
    (!debug.is_empty())
        .then(|| format!("\n{}", debug))
        .unwrap_or_default()
}

/// Formats the given [`Display`] implementor, then adds `indent`s to each line
/// to prettify the output.
fn format_display_with_indent<'d, D, I>(display: I, indent: usize) -> String
where
    D: Display + 'd,
    I: Into<Option<&'d D>>,
{
    let display = display
        .into()
        .map(|display| format!("{}", display))
        .unwrap_or_default()
        .lines()
        .map(|line| format!("{}{}", " ".repeat(indent), line))
        .join("\n");
    (!display.is_empty())
        .then(|| format!("\n{}", display))
        .unwrap_or_default()
}

/// Formats the given [`gherkin::Table`] and adds `indent`s to each line to
/// prettify the output.
fn format_table(table: &gherkin::Table, indent: usize) -> String {
    let max_row_len = table
        .rows
        .iter()
        .fold(None, |mut acc: Option<Vec<_>>, row| {
            if let Some(acc) = acc.as_mut() {
                for (cell, max_len) in row.iter().zip(acc) {
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
