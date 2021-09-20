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

use std::{fmt::Debug, ops::Deref};

use async_trait::async_trait;
use console::Term;
use itertools::Itertools as _;

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
}

#[async_trait(?Send)]
impl<W: World + Debug> Writer<W> for Basic {
    #[allow(clippy::unused_async)]
    async fn handle_event(&mut self, ev: parser::Result<event::Cucumber<W>>) {
        use event::{Cucumber, Feature, Rule};

        match ev {
            Err(err) => self.parsing_failed(&err),
            Ok(Cucumber::Started | Cucumber::Finished) => {}
            Ok(Cucumber::Feature(f, ev)) => match ev {
                Feature::Started => self.feature_started(&f),
                Feature::Scenario(sc, ev) => {
                    self.scenario(&f, &sc, &ev, 0);
                }
                Feature::Rule(r, ev) => match ev {
                    Rule::Started => {
                        self.rule_started(&r);
                    }
                    Rule::Scenario(sc, ev) => {
                        self.scenario(&f, &sc, &ev, 2);
                    }
                    Rule::Finished => {}
                },
                Feature::Finished => {}
            },
        }
    }
}

#[async_trait(?Send)]
impl<'val, W, Val> ArbitraryWriter<'val, W, Val> for Basic
where
    W: World + Debug,
    Val: AsRef<str> + 'val,
{
    #[allow(clippy::unused_async)]
    async fn write(&mut self, val: Val)
    where
        'val: 'async_trait,
    {
        self.write_line(val.as_ref()).unwrap();
    }
}

impl Default for Basic {
    fn default() -> Self {
        Self {
            terminal: Term::stdout(),
            styles: Styles::new(),
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
    fn clear_last_lines_if_term_present(&self, n: usize) {
        if self.styles.is_present {
            self.clear_last_lines(n).unwrap();
        }
    }

    /// Outputs [error] encountered while parsing some [`Feature`].
    ///
    /// [error]: event::Cucumber::ParsingError
    /// [`Feature`]: gherkin::Feature
    fn parsing_failed(&self, err: &gherkin::ParseFileError) {
        self.write_line(&self.styles.err(format!("Failed to parse: {}", err)))
            .unwrap();
    }

    /// Outputs [started] [`Feature`] to STDOUT.
    ///
    /// [started]: event::Feature::Started
    /// [`Feature`]: [`gherkin::Feature`]
    fn feature_started(&self, feature: &gherkin::Feature) {
        self.write_line(&self.styles.ok(format!("Feature: {}", feature.name)))
            .unwrap();
    }

    /// Outputs [started] [`Rule`] to STDOUT.
    ///
    /// [started]: event::Rule::Started
    /// [`Rule`]: [`gherkin::Rule`]
    fn rule_started(&self, rule: &gherkin::Rule) {
        self.write_line(&self.styles.ok(format!("  Rule: {}", rule.name)))
            .unwrap();
    }

    /// Outputs [`Scenario`] [started]/[background]/[step] event to STDOUT.
    ///
    /// [background]: event::Background
    /// [started]: event::Scenario::Started
    /// [step]: event::Step
    fn scenario<W: Debug>(
        &self,
        feat: &gherkin::Feature,
        scenario: &gherkin::Scenario,
        ev: &event::Scenario<W>,
        ident: usize,
    ) {
        use event::Scenario;

        let offset = ident + 2;
        match ev {
            Scenario::Started => {
                self.scenario_started(scenario, offset);
            }
            Scenario::Background(bg, ev) => {
                self.background(feat, bg, ev, offset);
            }
            Scenario::Step(st, ev) => {
                self.step(feat, st, ev, offset);
            }
            Scenario::Finished => {}
        }
    }

    /// Outputs [started] [`Scenario`] to STDOUT.
    ///
    /// [started]: event::Scenario::Started
    /// [`Scenario`]: [`gherkin::Scenario`]
    fn scenario_started(&self, scenario: &gherkin::Scenario, ident: usize) {
        self.write_line(&self.styles.ok(format!(
            "{}Scenario: {}",
            " ".repeat(ident),
            scenario.name,
        )))
        .unwrap();
    }

    /// Outputs [`Step`] [started]/[passed]/[skipped]/[failed] event to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Step`]: [`gherkin::Step`]
    fn step<W: Debug>(
        &self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        ev: &event::Step<W>,
        ident: usize,
    ) {
        use event::Step;

        let offset = ident + 4;
        match ev {
            Step::Started => {
                self.step_started(step, offset);
            }
            Step::Passed => {
                self.step_passed(step, offset);
            }
            Step::Skipped => {
                self.step_skipped(step, offset);
            }
            Step::Failed(world, info) => {
                self.step_failed(feat, step, world.as_ref(), info, offset);
            }
        }
    }

    /// Outputs [started] [`Step`] to STDOUT.
    ///
    /// This [`Step`] is printed only if terminal is present and gets
    /// overwritten by later [passed]/[skipped]/[failed] events.
    ///
    /// [started]: event::Step::Started
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Step`]: [`gherkin::Step`]
    fn step_started(&self, step: &gherkin::Step, ident: usize) {
        if self.styles.is_present {
            self.write_line(&format!(
                "{}{} {}",
                " ".repeat(ident),
                step.keyword,
                step.value,
            ))
            .unwrap();
        }
    }

    /// Outputs [passed] [`Step`] to STDOUT.
    ///
    /// [passed]: event::Step::Passed
    /// [`Step`]: [`gherkin::Step`]
    fn step_passed(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines_if_term_present(1);
        self.write_line(&self.styles.ok(format!(
            //  ✔
            "{}\u{2714}  {} {}",
            " ".repeat(ident - 3),
            step.keyword,
            step.value,
        )))
        .unwrap();
    }

    /// Outputs [skipped] [`Step`] to STDOUT.
    ///
    /// [skipped]: event::Step::Skipped
    /// [`Step`]: [`gherkin::Step`]
    fn step_skipped(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines_if_term_present(1);
        self.write_line(&self.styles.skipped(format!(
            "{}?  {} {} (skipped)",
            " ".repeat(ident - 3),
            step.keyword,
            step.value,
        )))
        .unwrap();
    }

    /// Outputs [failed] [`Step`] to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [`Step`]: [`gherkin::Step`]
    fn step_failed<W: Debug>(
        &self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        world: Option<&W>,
        info: &Info,
        ident: usize,
    ) {
        self.clear_last_lines_if_term_present(1);
        self.write_line(&self.styles.err(format!(
            //       ✘
            "{ident}\u{2718}  {} {}\n\
             {ident}   Step failed: {}:{}:{}\n\
             {ident}   Captured output: {}\
             {}",
            step.keyword,
            step.value,
            feat.path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            coerce_error(info),
            format_world(world, ident),
            ident = " ".repeat(ident - 3),
        )))
        .unwrap();
    }

    /// Outputs [`Background`] [`Step`] [started]/[passed]/[skipped]/[failed]
    /// event to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Background`]: [`gherkin::Background`]
    /// [`Step`]: [`gherkin::Step`]
    fn background<W: Debug>(
        &self,
        feat: &gherkin::Feature,
        bg: &gherkin::Step,
        ev: &event::Step<W>,
        ident: usize,
    ) {
        use event::Step;

        let offset = ident + 4;
        match ev {
            Step::Started => {
                self.bg_step_started(bg, offset);
            }
            Step::Passed => {
                self.bg_step_passed(bg, offset);
            }
            Step::Skipped => {
                self.bg_step_skipped(bg, offset);
            }
            Step::Failed(world, info) => {
                self.bg_step_failed(feat, bg, world.as_ref(), info, offset);
            }
        }
    }

    /// Outputs [started] [`Background`] [`Step`] to STDOUT.
    ///
    /// This [`Step`] is printed only if terminal is present and gets
    /// overwritten by later [passed]/[skipped]/[failed] events.
    ///
    /// [started]: event::Step::Started
    /// [passed]: event::Step::Passed
    /// [skipped]: event::Step::Skipped
    /// [started]: event::Step::Started
    /// [`Background`]: [`gherkin::Background`]
    /// [`Step`]: [`gherkin::Step`]
    fn bg_step_started(&self, step: &gherkin::Step, ident: usize) {
        if self.styles.is_present {
            self.write_line(&format!(
                "{}{}{} {}",
                " ".repeat(ident - 2),
                "> ",
                step.keyword,
                step.value,
            ))
            .unwrap();
        }
    }

    /// Outputs [passed] [`Background`] [`Step`] to STDOUT.
    ///
    /// [passed]: event::Step::Passed
    /// [`Background`]: [`gherkin::Background`]
    /// [`Step`]: [`gherkin::Step`]
    fn bg_step_passed(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines_if_term_present(1);
        self.write_line(&self.styles.ok(format!(
            //  ✔
            "{}\u{2714}> {} {}",
            " ".repeat(ident - 3),
            step.keyword,
            step.value,
        )))
        .unwrap();
    }

    /// Outputs [skipped] [`Background`] [`Step`] to STDOUT.
    ///
    /// [skipped]: event::Step::Skipped
    /// [`Background`]: [`gherkin::Background`]
    /// [`Step`]: [`gherkin::Step`]
    fn bg_step_skipped(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines_if_term_present(1);
        self.write_line(&self.styles.skipped(format!(
            "{}?> {} {} (skipped)",
            " ".repeat(ident - 3),
            step.keyword,
            step.value,
        )))
        .unwrap();
    }

    /// Outputs [failed] [`Background`] [`Step`] to STDOUT.
    ///
    /// [failed]: event::Step::Failed
    /// [`Background`]: [`gherkin::Background`]
    /// [`Step`]: [`gherkin::Step`]
    fn bg_step_failed<W: Debug>(
        &self,
        feat: &gherkin::Feature,
        step: &gherkin::Step,
        world: Option<&W>,
        info: &Info,
        ident: usize,
    ) {
        self.clear_last_lines_if_term_present(1);
        self.write_line(&self.styles.err(format!(
            //       ✘
            "{ident}\u{2718}> {} {}\n\
             {ident}   Background step failed: {}:{}:{}\n\
             {ident}   Captured output: {}\
             {}",
            step.keyword,
            step.value,
            feat.path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(&feat.name),
            step.position.line,
            step.position.col,
            coerce_error(info),
            format_world(world, ident),
            ident = " ".repeat(ident - 3),
        )))
        .unwrap();
    }
}

/// Tries to coerce [`catch_unwind()`] output to [`String`].
///
/// [`catch_unwind()`]: std::panic::catch_unwind()
#[must_use]
fn coerce_error(err: &Info) -> String {
    if let Some(string) = err.downcast_ref::<String>() {
        string.clone()
    } else if let Some(&string) = err.downcast_ref::<&str>() {
        string.to_owned()
    } else {
        "(Could not resolve panic payload)".to_owned()
    }
}

/// Formats the given [`World`] using [`Debug`], then adds `ident`s to each line
/// to prettify the output.
fn format_world<W: Debug>(world: Option<&W>, ident: usize) -> String {
    let world = world
        .map(|world| format!("{:#?}", world))
        .unwrap_or_default()
        .lines()
        .map(|line| format!("{}{}", " ".repeat(ident), line))
        .join("\n");
    (!world.is_empty())
        .then(|| format!("\n{}", world))
        .unwrap_or_default()
}
