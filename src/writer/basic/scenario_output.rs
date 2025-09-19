//! Scenario output handling for Basic writer.

use std::{fmt::Debug, io};

use crate::{
    event::{self, Info, Retries},
    writer::out::WriteStrExt as _,
};

use super::{basic_struct::Basic, formatting::{coerce_error, format_str_with_indent, trim_path}};

impl<Out: io::Write> Basic<Out> {
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
    pub(super) fn emit_log(&mut self, msg: impl AsRef<str>) -> io::Result<()> {
        self.lines_to_clear += self.styles.lines_count(msg.as_ref());
        self.re_output_after_clear.push_str(msg.as_ref());
        self.output.write_str(msg)
    }

    /// Outputs the [failed] [`Scenario`]'s hook.
    ///
    /// [failed]: event::Hook::Failed
    /// [`Scenario`]: gherkin::Scenario
    pub(super) fn hook_failed<W: Debug>(
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

        self.output.write_line(style(format!(
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
    pub(super) fn scenario_started(
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
            self.output.write_line(self.styles.retry(out))
        } else {
            let out = format!(
                "{}{}: {}",
                " ".repeat(self.indent),
                scenario.keyword,
                scenario.name,
            );
            self.output.write_line(self.styles.ok(out))
        }
    }
}

