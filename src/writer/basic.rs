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
use console::{Style, Term};
use itertools::Itertools as _;

use crate::{
    event::{self, Info},
    World, Writer,
};

/// Default [`Writer`] implementation.
#[derive(Clone, Debug)]
pub struct Basic {
    terminal: Term,
    ok: Style,
    skipped: Style,
    err: Style,
}

#[async_trait(?Send)]
impl<W: World + Debug> Writer<W> for Basic {
    async fn handle_event(&mut self, ev: event::Cucumber<W>) {
        match ev {
            event::Cucumber::Started | event::Cucumber::Finished => {}
            event::Cucumber::ParsingError(err) => self.parsing_failed(err),
            event::Cucumber::Feature(f, ev) => match ev {
                event::Feature::Started => self.feature_started(&f),
                event::Feature::Scenario(sc, ev) => {
                    self.scenario(&sc, &ev, 0);
                }
                event::Feature::Rule(r, ev) => match ev {
                    event::Rule::Started => {
                        self.rule_started(&r);
                    }
                    event::Rule::Scenario(sc, ev) => {
                        self.scenario(&sc, &ev, 2);
                    }
                    event::Rule::Finished => {}
                },
                event::Feature::Finished => {}
            },
        }
    }
}

impl Default for Basic {
    fn default() -> Self {
        Self {
            terminal: Term::stdout(),
            ok: Style::new().green(),
            skipped: Style::new().cyan(),
            err: Style::new().red(),
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
    /// Creates new [`Basic`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn parsing_failed(&self, err: gherkin::ParseFileError) {
        self.write_line(&format!(
            "{}",
            self.err.apply_to(format!("Failed to parse: {}", err))
        ))
        .unwrap();
    }

    fn feature_started(&self, feature: &gherkin::Feature) {
        self.write_line(&format!(
            "{}",
            self.ok.apply_to(format!("Feature: {}", feature.name))
        ))
        .unwrap();
    }

    fn rule_started(&self, rule: &gherkin::Rule) {
        self.write_line(&format!(
            "{}",
            self.ok.apply_to(format!("  Rule: {}", rule.name))
        ))
        .unwrap();
    }

    fn scenario<W: Debug>(
        &self,
        scenario: &gherkin::Scenario,
        ev: &event::Scenario<W>,
        ident: usize,
    ) {
        let offset = ident + 2;
        match ev {
            event::Scenario::Started => {
                self.scenario_started(scenario, offset);
            }
            event::Scenario::Background(bg, ev) => {
                self.background(bg, ev, offset);
            }
            event::Scenario::Step(st, ev) => {
                self.step(st, ev, offset);
            }
            event::Scenario::Finished => {}
        }
    }

    fn scenario_started(&self, scenario: &gherkin::Scenario, ident: usize) {
        self.write_line(&format!(
            "{}",
            self.ok.apply_to(format!(
                "{}Scenario: {}",
                " ".repeat(ident),
                scenario.name,
            ))
        ))
        .unwrap();
    }

    fn step<W: Debug>(
        &self,
        step: &gherkin::Step,
        ev: &event::Step<W>,
        ident: usize,
    ) {
        let offset = ident + 4;
        match ev {
            event::Step::Started => {
                self.step_started(step, offset);
            }
            event::Step::Passed => {
                self.step_passed(step, offset);
            }
            event::Step::Skipped => {
                self.step_skipped(step, offset);
            }
            event::Step::Failed(world, info) => {
                self.step_failed(step, world, info, offset);
            }
        }
    }

    fn step_started(&self, step: &gherkin::Step, ident: usize) {
        self.write_line(&format!(
            "{}{} {}",
            " ".repeat(ident),
            step.keyword,
            step.value,
        ))
        .unwrap();
    }

    fn step_passed(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines(1).unwrap();
        self.write_line(&format!(
            "{}",
            self.ok.apply_to(format!(
                //  ✔
                "{}\u{2714}  {} {}",
                " ".repeat(ident - 3),
                step.keyword,
                step.value,
            ))
        ))
        .unwrap();
    }

    fn step_skipped(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines(1).unwrap();
        self.write_line(&format!(
            "{}",
            self.skipped.apply_to(format!(
                "{}?  {} {} (skipped)",
                " ".repeat(ident - 3),
                step.keyword,
                step.value,
            ))
        ))
        .unwrap();
    }

    fn step_failed<W: Debug>(
        &self,
        step: &gherkin::Step,
        world: &W,
        info: &Info,
        ident: usize,
    ) {
        let world = format!("{:#?}", world)
            .lines()
            .map(|line| format!("{}{}\n", " ".repeat(ident), line))
            .join("");
        let world = world.trim_end_matches('\n');

        self.clear_last_lines(1).unwrap();
        self.write_line(&format!(
            "{}",
            self.err.apply_to(format!(
                //       ✘
                "{ident}\u{2718}  {} {}\n\
                 {ident}   Captured output: {}\n\
                 {}",
                step.keyword,
                step.value,
                coerce_error(info),
                world,
                ident = " ".repeat(ident - 3),
            ))
        ))
        .unwrap();
    }

    fn background<W: Debug>(
        &self,
        bg: &gherkin::Step,
        ev: &event::Step<W>,
        ident: usize,
    ) {
        let offset = ident + 4;
        match ev {
            event::Step::Started => {
                self.bg_step_started(bg, offset);
            }
            event::Step::Passed => {
                self.bg_step_passed(bg, offset);
            }
            event::Step::Skipped => {
                self.bg_step_skipped(bg, offset);
            }
            event::Step::Failed(world, info) => {
                self.bg_step_failed(bg, world, info, offset);
            }
        }
    }

    fn bg_step_started(&self, step: &gherkin::Step, ident: usize) {
        self.write_line(&format!(
            "{}{}{} {}",
            " ".repeat(ident - 2),
            "> ",
            step.keyword,
            step.value,
        ))
        .unwrap();
    }

    fn bg_step_passed(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines(1).unwrap();
        self.write_line(&format!(
            "{}",
            self.ok.apply_to(format!(
                //  ✔
                "{}\u{2714}> {} {}",
                " ".repeat(ident - 3),
                step.keyword,
                step.value,
            ))
        ))
        .unwrap();
    }

    fn bg_step_skipped(&self, step: &gherkin::Step, ident: usize) {
        self.clear_last_lines(1).unwrap();
        self.write_line(&format!(
            "{}",
            self.skipped.apply_to(format!(
                "{}?> {} {} (skipped)",
                " ".repeat(ident - 3),
                step.keyword,
                step.value,
            ))
        ))
        .unwrap();
    }

    fn bg_step_failed<W: Debug>(
        &self,
        step: &gherkin::Step,
        world: &W,
        info: &Info,
        ident: usize,
    ) {
        let world = format!("{:#?}", world)
            .lines()
            .map(|line| format!("{}{}\n", " ".repeat(ident), line))
            .join("");

        self.clear_last_lines(1).unwrap();
        self.write_line(&format!(
            "{}",
            self.err.apply_to(format!(
                //       ✘
                "{ident}\u{2718}> {} {}\n\
                 {ident}   Captured output: {}\n\
                 {}",
                step.keyword,
                step.value,
                coerce_error(info),
                world,
                ident = " ".repeat(ident - 3),
            ))
        ))
        .unwrap();
    }
}

fn coerce_error(err: &Info) -> String {
    if let Some(string) = err.downcast_ref::<String>() {
        string.clone()
    } else if let Some(&string) = err.downcast_ref::<&str>() {
        string.to_owned()
    } else {
        "(Could not resolve panic payload)".to_owned()
    }
}
