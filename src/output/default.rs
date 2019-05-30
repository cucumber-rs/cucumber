use std;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::Path;

use gherkin;
use pathdiff::diff_paths;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use textwrap;

use crate::OutputVisitor;
use crate::TestResult;

enum ScenarioResult {
    Pass,
    Fail,
    Skip,
}

pub struct DefaultOutput {
    stdout: StandardStream,
    cur_feature: String,
    feature_count: u32,
    feature_error_count: u32,
    rule_count: u32,
    scenarios: HashMap<gherkin::Scenario, ScenarioResult>,
    step_count: u32,
    skipped_count: u32,
    fail_count: u32,
}

impl std::default::Default for DefaultOutput {
    fn default() -> DefaultOutput {
        DefaultOutput {
            stdout: StandardStream::stdout(ColorChoice::Always),
            cur_feature: "".to_string(),
            feature_count: 0,
            feature_error_count: 0,
            rule_count: 0,
            scenarios: HashMap::new(),
            step_count: 0,
            skipped_count: 0,
            fail_count: 0,
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
    let comment_space = tw - c.chars().count() - 2;
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

impl DefaultOutput {
    fn set_color(&mut self, c: Color, b: bool) {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(c)).set_bold(b))
            .unwrap();
    }

    fn write(&mut self, s: &str, c: Color, bold: bool) {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(c)).set_bold(bold))
            .unwrap();
        write!(&mut self.stdout, "{}", s).unwrap();
        self.stdout
            .set_color(ColorSpec::new().set_fg(None).set_bold(false))
            .unwrap();
    }

    fn writeln(&mut self, s: &str, c: Color, bold: bool) {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(c)).set_bold(bold))
            .unwrap();
        writeln!(&mut self.stdout, "{}", s).unwrap();
        self.stdout
            .set_color(ColorSpec::new().set_fg(None).set_bold(false))
            .unwrap();
    }

    fn writeln_cmt(&mut self, s: &str, cmt: &str, indent: &str, c: Color, bold: bool) {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(c)).set_bold(bold))
            .unwrap();
        write!(&mut self.stdout, "{}", wrap_with_comment(s, cmt, indent)).unwrap();
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false))
            .unwrap();
        writeln!(&mut self.stdout, " {}", cmt).unwrap();
        self.stdout
            .set_color(ColorSpec::new().set_fg(None))
            .unwrap();
    }

    fn println(&mut self, s: &str) {
        writeln!(&mut self.stdout, "{}", s).unwrap();
    }

    fn red(&mut self, s: &str) {
        self.writeln(s, Color::Red, false);
    }

    fn bold_white(&mut self, s: &str) {
        self.writeln(s, Color::Green, true);
    }

    fn bold_white_comment(&mut self, s: &str, c: &str, indent: &str) {
        self.writeln_cmt(s, c, indent, Color::White, true);
    }

    fn relpath(&self, target: &Path) -> std::path::PathBuf {
        let target = target.canonicalize().expect("invalid target path");
        diff_paths(
            &target,
            &env::current_dir().expect("invalid current directory"),
        )
        .expect("invalid target path")
    }

    fn print_step_extras(&mut self, step: &gherkin::Step) {
        let indent = "      ";
        if let Some(ref table) = &step.table {
            // Find largest sized item per column
            let mut max_size: Vec<usize> = (&table.header).iter().map(|h| h.len()).collect();

            for row in &table.rows {
                for (n, field) in row.iter().enumerate() {
                    if field.len() > max_size[n] {
                        max_size[n] = field.len();
                    }
                }
            }

            // If number print in a number way
            let formatted_header_fields: Vec<String> = (&table.header)
                .iter()
                .enumerate()
                .map(|(n, field)| format!(" {: <1$} ", field, max_size[n]))
                .collect();

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

            print!("{}", indent);
            let border_color = Color::Magenta;
            self.write("|", border_color, true);
            for field in formatted_header_fields {
                self.write(&field, Color::White, true);
                self.write("|", border_color, true);
            }
            self.println("");

            for row in formatted_row_fields {
                print!("{}", indent);
                self.write("|", border_color, false);
                for field in row {
                    print!("{}", field);
                    self.write("|", border_color, false);
                }
                self.println("");
            }
        };

        if let Some(ref docstring) = &step.docstring {
            self.writeln(&format!("{}\"\"\"", indent), Color::Magenta, true);
            println!("{}", textwrap::indent(docstring, indent).trim_end());
            self.writeln(&format!("{}\"\"\"", indent), Color::Magenta, true);
        }
    }

    fn print_finish(&mut self) -> Result<(), std::io::Error> {
        self.set_color(Color::White, true);

        // Do feature count
        write!(&mut self.stdout, "{} features", &self.feature_count)?;
        if self.feature_error_count > 0 {
            write!(&mut self.stdout, " (")?;
            self.set_color(Color::Red, true);
            write!(&mut self.stdout, "{} errored", self.feature_error_count)?;
            self.set_color(Color::White, true);
            write!(&mut self.stdout, ")")?;
        }

        // Do rule count
        if self.rule_count > 0 {
            write!(&mut self.stdout, ", {} rules", &self.rule_count)?;
        }

        self.println("");

        // Do scenario count
        let scenario_passed_count = self
            .scenarios
            .values()
            .filter(|v| match v {
                ScenarioResult::Pass => true,
                _ => false,
            })
            .count();
        let scenario_fail_count = self
            .scenarios
            .values()
            .filter(|v| match v {
                ScenarioResult::Fail => true,
                _ => false,
            })
            .count();
        let scenario_skipped_count = self
            .scenarios
            .values()
            .filter(|v| match v {
                ScenarioResult::Skip => true,
                _ => false,
            })
            .count();

        write!(&mut self.stdout, "{} scenarios (", &self.scenarios.len())?;

        if scenario_fail_count > 0 {
            self.set_color(Color::Red, true);
            write!(&mut self.stdout, "{} failed", scenario_fail_count)?;
            self.set_color(Color::White, true);
        }

        if scenario_skipped_count > 0 {
            if scenario_fail_count > 0 {
                write!(&mut self.stdout, ", ")?;
            }
            self.set_color(Color::Cyan, true);
            write!(&mut self.stdout, "{} skipped", scenario_skipped_count)?;
            self.set_color(Color::White, true);
        }

        if scenario_fail_count > 0 || scenario_skipped_count > 0 {
            write!(&mut self.stdout, ", ")?;
        }

        self.set_color(Color::Green, true);
        write!(&mut self.stdout, "{} passed", scenario_passed_count)?;
        self.set_color(Color::White, true);

        write!(&mut self.stdout, ")")?;

        self.println("");

        // Do steps
        let passed_count = self.step_count - self.skipped_count - self.fail_count;

        write!(&mut self.stdout, "{} steps (", &self.step_count)?;

        if self.fail_count > 0 {
            self.set_color(Color::Red, true);
            write!(&mut self.stdout, "{} failed", self.fail_count)?;
            self.set_color(Color::White, true);
        }

        if self.skipped_count > 0 {
            if self.fail_count > 0 {
                write!(&mut self.stdout, ", ")?;
            }
            self.set_color(Color::Cyan, true);
            write!(&mut self.stdout, "{} skipped", self.skipped_count)?;
            self.set_color(Color::White, true);
        }

        if self.fail_count > 0 || self.skipped_count > 0 {
            write!(&mut self.stdout, ", ")?;
        }

        self.set_color(Color::Green, true);
        write!(&mut self.stdout, "{} passed", passed_count)?;
        self.set_color(Color::White, true);
        write!(&mut self.stdout, ")")?;
        self.println("");

        self.stdout
            .set_color(ColorSpec::new().set_fg(None).set_bold(false))?;
        self.println("");

        Ok(())
    }
}

impl OutputVisitor for DefaultOutput {
    fn visit_start(&mut self) {
        self.bold_white(&format!("[Cucumber v{}]\n", env!("CARGO_PKG_VERSION")))
    }

    fn visit_feature(&mut self, feature: &gherkin::Feature, path: &Path) {
        self.cur_feature = self.relpath(&path).to_string_lossy().to_string();
        let msg = &format!("Feature: {}", &feature.name);
        let cmt = &format!(
            "{}:{}:{}",
            &self.cur_feature, feature.position.0, feature.position.1
        );
        self.bold_white_comment(msg, cmt, "");
        println!();

        self.feature_count += 1;
    }

    fn visit_feature_end(&mut self, _feature: &gherkin::Feature) {}

    fn visit_feature_error<'r>(&mut self, path: &Path, error: &gherkin::Error<'r>) {
        let position = gherkin::error_position(error);
        let relpath = self.relpath(&path).to_string_lossy().to_string();
        let loc = &format!("{}:{}:{}", &relpath, position.0, position.1);

        self.writeln_cmt(
            &format!(
                "{:—<1$}",
                "! Parsing feature failed: ",
                textwrap::termwidth() - loc.chars().count() - 7
            ),
            &loc,
            "———— ",
            Color::Red,
            true,
        );

        self.red(
            &textwrap::indent(
                &textwrap::fill(&format!("{}", error), textwrap::termwidth() - 4),
                "  ",
            )
            .trim_end(),
        );

        self.writeln(
            &format!("{:—<1$}\n", "", textwrap::termwidth()),
            Color::Red,
            true,
        );

        self.feature_error_count += 1;
    }

    fn visit_rule(&mut self, rule: &gherkin::Rule) {
        let cmt = &format!(
            "{}:{}:{}",
            &self.cur_feature, rule.position.0, rule.position.1
        );
        self.bold_white_comment(&format!("Rule: {}\n", &rule.name), cmt, " ");
    }

    fn visit_rule_end(&mut self, _rule: &gherkin::Rule) {
        self.rule_count += 1;
    }

    fn visit_scenario(&mut self, rule: Option<&gherkin::Rule>, scenario: &gherkin::Scenario) {
        let cmt = &format!(
            "{}:{}:{}",
            &self.cur_feature, scenario.position.0, scenario.position.1
        );
        let indent = if rule.is_some() { "  " } else { " " };
        self.bold_white_comment(&format!("Scenario: {}", &scenario.name), cmt, indent);
    }

    fn visit_scenario_skipped(
        &mut self,
        _rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
    ) {
        if !self.scenarios.contains_key(scenario) {
            self.scenarios
                .insert(scenario.clone(), ScenarioResult::Skip);
        }
    }

    fn visit_scenario_end(&mut self, _rule: Option<&gherkin::Rule>, scenario: &gherkin::Scenario) {
        if !self.scenarios.contains_key(scenario) {
            self.scenarios
                .insert(scenario.clone(), ScenarioResult::Pass);
        }
        self.println("");
    }

    fn visit_step(
        &mut self,
        _rule: Option<&gherkin::Rule>,
        _scenario: &gherkin::Scenario,
        _step: &gherkin::Step,
    ) {
        self.step_count += 1;
    }

    fn visit_step_result(
        &mut self,
        rule: Option<&gherkin::Rule>,
        scenario: &gherkin::Scenario,
        step: &gherkin::Step,
        result: &TestResult,
    ) {
        let cmt = &format!(
            "{}:{}:{}",
            &self.cur_feature, step.position.0, step.position.1
        );
        let msg = &step.to_string();
        let indent = if rule.is_some() { "   " } else { "  " };

        match result {
            TestResult::Pass => {
                self.writeln_cmt(&format!("✔ {}", msg), cmt, indent, Color::Green, false);
                self.print_step_extras(step);
            }
            TestResult::Fail(panic_info, captured_output) => {
                self.writeln_cmt(&format!("✘ {}", msg), cmt, indent, Color::Red, false);
                self.print_step_extras(step);
                self.writeln_cmt(
                    &format!(
                        "{:—<1$}",
                        "! Step failed: ",
                        textwrap::termwidth() - panic_info.location.chars().count() - 7
                    ),
                    &panic_info.location,
                    "———— ",
                    Color::Red,
                    true,
                );
                self.red(
                    &textwrap::indent(
                        &textwrap::fill(&panic_info.payload, textwrap::termwidth() - 4),
                        "  ",
                    )
                    .trim_end(),
                );

                if !captured_output.is_empty() {
                    self.writeln(
                        &format!(
                            "{:—<1$}",
                            "———— Captured output: ",
                            textwrap::termwidth()
                        ),
                        Color::Red,
                        true,
                    );
                    let output_str = String::from_utf8(captured_output.to_vec())
                        .unwrap_or_else(|_| format!("{:?}", captured_output));
                    self.red(
                        &textwrap::indent(
                            &textwrap::fill(&output_str, textwrap::termwidth() - 4),
                            "  ",
                        )
                        .trim_end(),
                    );
                }
                self.writeln(
                    &format!("{:—<1$}", "", textwrap::termwidth()),
                    Color::Red,
                    true,
                );

                self.fail_count += 1;
                self.scenarios
                    .insert(scenario.clone(), ScenarioResult::Fail);
            }
            TestResult::MutexPoisoned => {
                self.writeln_cmt(&format!("- {}", msg), cmt, indent, Color::Cyan, false);
                self.print_step_extras(step);
                self.write(&format!("{}  ⚡ ", indent), Color::Yellow, false);
                self.println("Skipped due to previous error (poisoned)");
                self.fail_count += 1;
            }
            TestResult::Skipped => {
                self.writeln_cmt(&format!("- {}", msg), cmt, indent, Color::Cyan, false);
                self.print_step_extras(step);
                self.skipped_count += 1;
            }
            TestResult::Unimplemented => {
                self.writeln_cmt(&format!("- {}", msg), cmt, indent, Color::Cyan, false);
                self.print_step_extras(step);
                self.write(&format!("{}  ⚡ ", indent), Color::Yellow, false);
                self.println("Not yet implemented (skipped)");

                self.skipped_count += 1;
            }
        };
    }

    fn visit_finish(&mut self) {
        self.print_finish().unwrap();
    }
}
