//! Feature and Rule output handling for Basic writer.

use std::{fmt::Debug, io};

use crate::{
    event, 
    writer::out::WriteStrExt as _,
};

use super::basic_struct::Basic;

impl<Out: io::Write> Basic<Out> {
    /// Outputs the [started] [`Feature`].
    ///
    /// [started]: event::Feature::Started
    /// [`Feature`]: gherkin::Feature
    pub(super) fn feature_started(
        &mut self,
        feature: &gherkin::Feature,
    ) -> io::Result<()> {
        let out = format!("{}: {}", feature.keyword, feature.name);
        self.output.write_line(self.styles.ok(out))
    }

    /// Outputs the [`Rule`]'s [started]/[scenario]/[finished] event.
    ///
    /// [finished]: event::Rule::Finished
    /// [scenario]: event::Rule::Scenario
    /// [started]: event::Rule::Started
    /// [`Rule`]: gherkin::Rule
    pub(super) fn rule<W: Debug>(
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
    pub(super) fn rule_started(
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
        self.output.write_line(self.styles.ok(out))
    }
}

