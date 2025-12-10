//! Background step output handling for Basic writer.

use std::{fmt::Debug, io};

use regex::CaptureLocations;

use crate::{
    event::{self, Retries},
    step,
    writer::out::WriteStrExt as _,
};

use super::{
    basic_struct::Basic,
    formatting::{format_captures, format_str_with_indent, format_table, trim_path},
};

impl<Out: io::Write> Basic<Out> {
    /// Outputs the [`Background`] [`Step`]'s
    /// [started]/[passed]/[skipped]/[failed] event.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub(super) fn background<W: Debug>(
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
            Step::Passed { captures, .. } => {
                self.bg_step_passed(sc, bg, &captures, retries)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Skipped => {
                self.bg_step_skipped(feat, bg)?;
                self.indent = self.indent.saturating_sub(4);
            }
            Step::Failed { captures, location, world, error } => {
                self.bg_step_failed(
                    feat,
                    bg,
                    captures.as_ref(),
                    *location,
                    retries,
                    world.as_ref(),
                    error,
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
    pub(super) fn bg_step_started(
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
    pub(super) fn bg_step_passed(
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

        self.output.write_line(style(format!(
            "{step_keyword}{step_value}{doc_str}{step_table}",
        )))
    }

    /// Outputs the [skipped] [`Background`] [`Step`].
    ///
    /// [skipped]: event::Step::Skipped
    /// [`Background`]: gherkin::Background
    /// [`Step`]: gherkin::Step
    pub(super) fn bg_step_skipped(
        &mut self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
    ) -> io::Result<()> {
        self.clear_last_lines_if_term_present()?;
        self.output.write_line(self.styles.skipped(format!(
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
    pub(super) fn bg_step_failed<W: Debug>(
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
            .write_line(format!("{step_keyword}{step_value}{diagnostics}"))
    }
}

