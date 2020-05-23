// Copyright (c) 2018-2020  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::PathBuf;
use std::rc::Rc;

use crate::{
    event::{CucumberEvent, RuleEvent, ScenarioEvent, StepEvent},
    EventHandler,
};
use gherkin::{Feature, Rule, Scenario, Step};

#[derive(Debug, Clone, Default)]
struct Counter {
    total: u32,
    skipped: u32,
    passed: u32,
    failed: u32,
}

pub struct BasicOutput {
    features: Counter,
    rules: Counter,
    scenarios: Counter,
    steps: Counter,
    started: std::time::Instant,
}

impl Default for BasicOutput {
    fn default() -> BasicOutput {
        BasicOutput {
            features: Default::default(),
            rules: Default::default(),
            scenarios: Default::default(),
            steps: Default::default(),
            started: std::time::Instant::now(),
        }
    }
}

fn wrap_with_comment(s: &str, c: &str, indent: &str) -> String {
    let tw = textwrap::termwidth();
    let w = tw - indent.chars().count();
    let mut cs: Vec<String> = textwrap::wrap_iter(s, w)
        .map(|x| format!("{}{}", indent, &x.trim()))
        .collect();
    // Fit the comment onto the last line
    let comment_space = tw.saturating_sub(c.chars().count()).saturating_sub(2);
    let last_count = cs.last().unwrap().chars().count();
    if last_count > comment_space {
        cs.push(format!("{: <1$}", "", comment_space))
    } else {
        cs.last_mut()
            .unwrap()
            .push_str(&format!("{: <1$}", "", comment_space - last_count));
    }
    cs.join("\n")
}

impl BasicOutput {
    fn relpath(&self, target: Option<&std::path::PathBuf>) -> String {
        let target = match target {
            Some(v) => v,
            None => return "<unknown>".into(),
        };
        let target = target.canonicalize().expect("invalid target path");
        pathdiff::diff_paths(
            &target,
            &std::env::current_dir().expect("invalid current directory"),
        )
        .expect("invalid target path")
        .to_string_lossy()
        .to_string()
    }

    fn print_step_extras(&mut self, step: &gherkin::Step) {
        let indent = "      ";
        if let Some(ref table) = &step.table {
            // Find largest sized item per column
            let mut max_size: Vec<usize> = vec![0; table.row_width()];

            for row in &table.rows {
                for (n, field) in row.iter().enumerate() {
                    if field.len() > max_size[n] {
                        max_size[n] = field.len();
                    }
                }
            }

            let formatted_row_fields: Vec<Vec<String>> = (&table.rows)
                .iter()
                .map(|row| {
                    row.iter()
                        .enumerate()
                        .map(|(n, field)| {
                            if field.parse::<f64>().is_ok() {
                                format!(" {: >1$} ", field, max_size[n])
                            } else {
                                format!(" {: <1$} ", field, max_size[n])
                            }
                        })
                        .collect()
                })
                .collect();

            let border_color = termcolor::Color::Magenta;

            for row in formatted_row_fields {
                print!("{}", indent);
                self.write("|", border_color, false);
                for field in row {
                    print!("{}", field);
                    self.write("|", border_color, false);
                }
                println!("");
            }
        };

        if let Some(ref docstring) = &step.docstring {
            self.writeln(
                &format!("{}\"\"\"", indent),
                termcolor::Color::Magenta,
                true,
            );
            println!("{}", textwrap::indent(docstring, indent).trim_end());
            self.writeln(
                &format!("{}\"\"\"", indent),
                termcolor::Color::Magenta,
                true,
            );
        }
    }

    fn write(&mut self, s: &str, c: termcolor::Color, bold: bool) {
        if bold {
            cprint!(bold c, "{}", s);
        } else {
            cprint!(c, "{}", s);
        }
    }

    fn writeln(&mut self, s: &str, c: termcolor::Color, bold: bool) {
        if bold {
            cprintln!(bold c, "{}", s);
        } else {
            cprintln!(c, "{}", s);
        }
    }

    fn writeln_cmt(&mut self, s: &str, cmt: &str, indent: &str, c: termcolor::Color, bold: bool) {
        if bold {
            cprint!(bold c, "{}", wrap_with_comment(s, cmt, indent));
        } else {
            cprint!(c, "{}", wrap_with_comment(s, cmt, indent));
        }
        cprintln!(termcolor::Color::White, " {}", cmt);
    }

    fn file_line_col(&self, file: Option<&PathBuf>, position: (usize, usize)) -> String {
        match file {
            Some(v) => format!("{}:{}:{}", self.relpath(Some(v)), position.0, position.1),
            None => format!("<input>:{}:{}", position.0, position.1),
        }
    }

    fn handle_step(
        &mut self,
        feature: Rc<Feature>,
        rule: Option<Rc<Rule>>,
        _scenario: Rc<Scenario>,
        step: Rc<Step>,
        event: StepEvent,
        is_bg: bool,
    ) {
        self.steps.total += 1;

        let cmt = self.file_line_col(feature.path.as_ref(), step.position);
        let msg = if is_bg {
            format!("(Background) {}", &step)
        } else {
            step.to_string()
        };
        let indent = if rule.is_some() { "   " } else { "  " };

        match event {
            StepEvent::Unimplemented => {
                self.steps.skipped += 1;

                self.writeln_cmt(
                    &format!("- {}", msg),
                    &cmt,
                    indent,
                    termcolor::Color::Cyan,
                    false,
                );
                self.print_step_extras(&*step);
                self.write(&format!("{}  ⚡ ", indent), termcolor::Color::Yellow, false);
                println!("Not yet implemented (skipped)");
            }
            StepEvent::Skipped => {
                self.steps.skipped += 1;

                self.writeln_cmt(
                    &format!("- {}", msg),
                    &cmt,
                    indent,
                    termcolor::Color::Cyan,
                    false,
                );
                self.print_step_extras(&*step);
            }
            StepEvent::Passed(_output) => {
                self.steps.passed += 1;

                self.writeln_cmt(
                    &format!("✔ {}", msg),
                    &cmt,
                    indent,
                    termcolor::Color::Green,
                    false,
                );
                self.print_step_extras(&*step);
            }
            StepEvent::Failed(output, panic_info) => {
                self.steps.failed += 1;

                self.writeln_cmt(
                    &format!("✘ {}", msg),
                    &cmt,
                    indent,
                    termcolor::Color::Red,
                    false,
                );
                self.print_step_extras(&*step);
                self.writeln_cmt(
                    &format!(
                        "{:—<1$}",
                        "[!] Step failed: ",
                        textwrap::termwidth()
                            .saturating_sub(panic_info.location.to_string().chars().count())
                            .saturating_sub(7),
                    ),
                    &panic_info.location.to_string(),
                    "———— ",
                    termcolor::Color::Red,
                    true,
                );
                self.writeln(
                    &textwrap::indent(
                        &textwrap::fill(&panic_info.payload, textwrap::termwidth() - 4),
                        "  ",
                    )
                    .trim_end(),
                    termcolor::Color::Red,
                    false,
                );

                if !output.out.is_empty() {
                    self.writeln(
                        &format!("{:—<1$}", "———— Captured stdout: ", textwrap::termwidth()),
                        termcolor::Color::Red,
                        true,
                    );

                    self.writeln(
                        &textwrap::indent(
                            &textwrap::fill(&output.out, textwrap::termwidth() - 4),
                            "  ",
                        )
                        .trim_end(),
                        termcolor::Color::Red,
                        false,
                    );
                }

                if !output.err.is_empty() {
                    self.writeln(
                        &format!("{:—<1$}", "———— Captured stderr: ", textwrap::termwidth()),
                        termcolor::Color::Red,
                        true,
                    );

                    self.writeln(
                        &textwrap::indent(
                            &textwrap::fill(&output.err, textwrap::termwidth() - 4),
                            "  ",
                        )
                        .trim_end(),
                        termcolor::Color::Red,
                        false,
                    );
                }

                self.writeln(
                    &format!("{:—<1$}", "", textwrap::termwidth()),
                    termcolor::Color::Red,
                    true,
                );
            }
        }
    }

    fn handle_scenario(
        &mut self,
        feature: Rc<Feature>,
        rule: Option<Rc<Rule>>,
        scenario: Rc<Scenario>,
        event: ScenarioEvent,
    ) {
        match event {
            ScenarioEvent::Starting => {
                self.scenarios.total += 1;
                let cmt = self.file_line_col(feature.path.as_ref(), scenario.position);
                let indent = if rule.is_some() { "  " } else { " " };
                self.writeln_cmt(
                    &format!("Scenario: {}", &scenario.name),
                    &cmt,
                    indent,
                    termcolor::Color::White,
                    true,
                );
            }
            ScenarioEvent::Background(step, event) => {
                self.handle_step(feature, rule, scenario, step, event, true)
            }
            ScenarioEvent::Step(step, event) => {
                self.handle_step(feature, rule, scenario, step, event, false)
            }
            ScenarioEvent::Skipped => {
                self.scenarios.skipped += 1;
            }
            ScenarioEvent::Passed => {
                self.scenarios.passed += 1;
            }
            ScenarioEvent::Failed => {
                self.scenarios.failed += 1;
            }
        }
    }

    fn handle_rule(&mut self, feature: Rc<Feature>, rule: Rc<Rule>, event: RuleEvent) {
        match event {
            RuleEvent::Starting => {
                self.rules.total += 1;

                let cmt = self.file_line_col(feature.path.as_ref(), rule.position);
                self.writeln_cmt(
                    &format!("Rule: {}", &rule.name),
                    &cmt,
                    " ",
                    termcolor::Color::White,
                    true,
                );
            }
            RuleEvent::Scenario(scenario, event) => {
                self.handle_scenario(feature, Some(rule), scenario, event)
            }
            RuleEvent::Skipped => {
                self.rules.skipped += 1;
            }
            RuleEvent::Passed => {
                self.rules.passed += 1;
            }
            RuleEvent::Failed => {
                self.rules.failed += 1;
            }
        }
    }

    fn print_counter(&self, name: &str, counter: &Counter) {
        use termcolor::Color::*;

        cprint!(bold White, "{} {} (", counter.total, name);

        if counter.failed > 0 {
            cprint!(bold Red, "{} failed", counter.failed);
        }

        if counter.skipped > 0 {
            if counter.failed > 0 {
                cprint!(bold White, ", ");
            }
            cprint!(bold Cyan, "{} skipped", counter.skipped);
        }

        if counter.failed > 0 || counter.skipped > 0 {
            cprint!(bold White, ", ");
        }

        cprint!(bold Green, "{} passed", counter.passed);
        cprintln!(bold White, ")");
    }

    fn print_finish(&self) {
        use termcolor::Color::*;

        cprintln!(bold Blue, "[Summary]");
        cprintln!(bold White, "{} features", self.features.total);

        self.print_counter("scenarios", &self.scenarios);
        if self.rules.total > 0 {
            self.print_counter("rules", &self.rules);
        }
        self.print_counter("steps", &self.steps);

        let t = self.started.elapsed();
        println!(
            "\nFinished in {}.{} seconds.",
            t.as_secs(),
            t.subsec_millis()
        );
    }
}

impl EventHandler for BasicOutput {
    fn handle_event(&mut self, event: CucumberEvent) {
        match event {
            CucumberEvent::Starting => {
                self.started = std::time::Instant::now();
                cprintln!(bold termcolor::Color::Blue, "[Cucumber v{}]", env!("CARGO_PKG_VERSION"))
            }
            CucumberEvent::Finished => self.print_finish(),
            CucumberEvent::Feature(feature, event) => match event {
                crate::event::FeatureEvent::Starting => {
                    self.features.total += 1;

                    let msg = &format!("Feature: {}", &feature.name);
                    let cmt = self.file_line_col(feature.path.as_ref(), feature.position);
                    self.writeln_cmt(msg, &cmt, "", termcolor::Color::White, true);
                    println!();
                }
                crate::event::FeatureEvent::Scenario(scenario, event) => {
                    self.handle_scenario(feature, None, scenario, event)
                }
                crate::event::FeatureEvent::Rule(rule, event) => {
                    self.handle_rule(feature, rule, event)
                }
                crate::event::FeatureEvent::Finished => {
                    println!();
                }
            },
        }
    }
}
