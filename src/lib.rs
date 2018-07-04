// Copyright (c) 2018  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(set_stdio)]
#![feature(fnbox)]

pub extern crate gherkin_rust as gherkin;
pub extern crate regex;
extern crate termcolor;
extern crate pathdiff;
extern crate textwrap;

use gherkin::{Step, StepType, Feature};
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io;
use std::io::prelude::*;
use std::ops::Deref;
use std::panic;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::any::Any;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use pathdiff::diff_paths;
use std::env;

pub trait World: Default {}

pub trait OutputVisitor : Default {
    fn visit_start(&mut self);
    fn visit_feature(&mut self, feature: &gherkin::Feature, path: &Path);
    fn visit_feature_end(&mut self, feature: &gherkin::Feature);
    fn visit_feature_error<'a>(&mut self, path: &Path, error: &gherkin::Error<'a>);
    fn visit_scenario(&mut self, scenario: &gherkin::Scenario);
    fn visit_scenario_end(&mut self, scenario: &gherkin::Scenario);
    fn visit_scenario_skipped(&mut self, scenario: &gherkin::Scenario);
    fn visit_step(&mut self, scenario: &gherkin::Scenario, step: &gherkin::Step);
    fn visit_step_result(&mut self, scenario: &gherkin::Scenario, step: &gherkin::Step, result: &TestResult);
    fn visit_finish(&mut self);
}

enum ScenarioResult {
    Pass,
    Fail,
    Skip
}

pub struct DefaultOutput {
    stdout: StandardStream,
    cur_feature: String,
    feature_count: u32,
    feature_error_count: u32,
    scenarios: HashMap<gherkin::Scenario, ScenarioResult>,
    step_count: u32,
    skipped_count: u32,
    fail_count: u32
}

impl std::default::Default for DefaultOutput {
    fn default() -> DefaultOutput {
        DefaultOutput {
            stdout: StandardStream::stdout(ColorChoice::Always),
            cur_feature: "".to_string(),
            feature_count: 0,
            feature_error_count: 0,
            scenarios: HashMap::new(),
            step_count: 0,
            skipped_count: 0,
            fail_count: 0
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
        cs.last_mut().unwrap().push_str(&format!("{: <1$}", "", comment_space - last_count));
    }
    cs.join("\n")
}

impl DefaultOutput {
    fn set_color(&mut self, c: Color, b: bool) {
        self.stdout.set_color(ColorSpec::new()
            .set_fg(Some(c))
            .set_bold(b)).unwrap();
    }

    fn write(&mut self, s: &str, c: Color, bold: bool) {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(c)).set_bold(bold)).unwrap();
        write!(&mut self.stdout, "{}", s).unwrap();
        self.stdout.set_color(ColorSpec::new().set_fg(None).set_bold(false)).unwrap();
    }

    fn writeln(&mut self, s: &str, c: Color, bold: bool) {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(c)).set_bold(bold)).unwrap();
        writeln!(&mut self.stdout, "{}", s).unwrap();
        self.stdout.set_color(ColorSpec::new().set_fg(None).set_bold(false)).unwrap();
    }

    fn writeln_cmt(&mut self, s: &str, cmt: &str, indent: &str, c: Color, bold: bool) {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(c)).set_bold(bold)).unwrap();
        write!(&mut self.stdout, "{}", wrap_with_comment(s, cmt, indent)).unwrap();
        self.stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false)).unwrap();
        writeln!(&mut self.stdout, " {}", cmt).unwrap();
        self.stdout.set_color(ColorSpec::new().set_fg(None)).unwrap();
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
        diff_paths(&target, &env::current_dir().unwrap()).unwrap()
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
            let formatted_header_fields: Vec<String> = (&table.header).iter()
                .enumerate()
                .map(|(n, field)| {
                    format!(" {: <1$} ", field, max_size[n])
                }).collect();

            let formatted_row_fields: Vec<Vec<String>> = (&table.rows).iter()
                .map(|row| {
                    row.iter().enumerate().map(|(n, field)| {
                        if field.parse::<f64>().is_ok() {
                            format!(" {: >1$} ", field, max_size[n])
                        } else {
                            format!(" {: <1$} ", field, max_size[n])
                        }
                    }).collect()
                }).collect();

            print!("{}", indent);
            let border_color = Color::Magenta;
            self.write("|", border_color.clone(), true);
            for field in formatted_header_fields {
                self.write(&field, Color::White, true);
                self.write("|", border_color.clone(), true);
            }
            println!();

            for row in formatted_row_fields {
                print!("{}", indent);
                self.write("|", border_color.clone(), false);
                for field in row {
                    print!("{}", field);
                    self.write("|", border_color.clone(), false);
                }
                println!();
            }
        };

        if let Some(ref docstring) = &step.docstring {
            self.writeln(&format!("{}\"\"\"", indent), Color::Magenta, true);
            println!("{}", textwrap::indent(docstring, indent).trim_right());
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

        println!();
            
        // Do scenario count
        let scenario_passed_count = self.scenarios.values().filter(|v| {
            match v {
                ScenarioResult::Pass => true,
                _ => false
            }
        }).count();
        let scenario_fail_count = self.scenarios.values().filter(|v| {
            match v {
                ScenarioResult::Fail => true,
                _ => false
            }
        }).count();
        let scenario_skipped_count = self.scenarios.values().filter(|v| {
            match v {
                ScenarioResult::Skip => true,
                _ => false
            }
        }).count();
        
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

        println!();

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
        println!();

        self.stdout.set_color(ColorSpec::new()
            .set_fg(None)
            .set_bold(false))?;
        println!();

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
        let cmt = &format!("{}:{}:{}", &self.cur_feature, feature.position.0, feature.position.1);
        self.bold_white_comment(msg, cmt, "");
        println!("");

        self.feature_count += 1;
    }
    
    fn visit_feature_end(&mut self, _feature: &gherkin::Feature) {}

    fn visit_feature_error<'r>(&mut self, path: &Path, error: &gherkin::Error<'r>) {
        let position = gherkin::error_position(error);
        let relpath = self.relpath(&path).to_string_lossy().to_string();
        let loc = &format!("{}:{}:{}", &relpath, position.0, position.1);

        self.writeln_cmt(
            &format!(
                "{:—<1$}", "! Parsing feature failed: ",
                textwrap::termwidth() - loc.chars().count() - 7
            ),
            &loc,
            "———— ",
            Color::Red,
            true);
        
        self.red(
            &textwrap::indent(
                &textwrap::fill(
                    &format!("{}", error),
                    textwrap::termwidth() - 4
                ),
                "  "
            ).trim_right()
        );

        self.writeln(&format!("{:-<1$}\n", "", textwrap::termwidth()), Color::Red, true);

        self.feature_error_count += 1;  
    }

    fn visit_scenario(&mut self, scenario: &gherkin::Scenario) {
        let cmt = &format!("{}:{}:{}", &self.cur_feature, scenario.position.0, scenario.position.1);
        self.bold_white_comment(&format!("Scenario: {}", &scenario.name), cmt, " ");
    }

    fn visit_scenario_skipped(&mut self, scenario: &gherkin::Scenario) {
        if !self.scenarios.contains_key(scenario) {
            self.scenarios.insert(scenario.clone(), ScenarioResult::Skip);
        }
    }
    
    fn visit_scenario_end(&mut self, scenario: &gherkin::Scenario) {
        if !self.scenarios.contains_key(scenario) {
            self.scenarios.insert(scenario.clone(), ScenarioResult::Pass);
        }
        println!();
    }
    
    fn visit_step(&mut self, _scenario: &gherkin::Scenario, _step: &gherkin::Step) {
        self.step_count += 1;
    }
    
    fn visit_step_result(&mut self, scenario: &gherkin::Scenario, step: &gherkin::Step, result: &TestResult) {
        let cmt = &format!("{}:{}:{}", &self.cur_feature, step.position.0, step.position.1);
        let msg = &format!("{}", &step.to_string());
        let indent = "  ";

        match result {
            TestResult::Pass => {
                self.writeln_cmt(&format!("✔ {}", msg), cmt, indent, Color::Green, false);
                self.print_step_extras(step);
            },
            TestResult::Fail(err_msg, loc) => {
                self.writeln_cmt(&format!("✘ {}", msg), cmt, indent, Color::Red, false);
                self.print_step_extras(step);
                self.writeln_cmt(
                    &format!(
                        "{:—<1$}", "! Step failed: ",
                        textwrap::termwidth() - loc.chars().count() - 7
                    ),
                    loc,
                    "———— ",
                    Color::Red,
                    true);
                self.red(&textwrap::indent(&textwrap::fill(err_msg, textwrap::termwidth() - 4), "  ").trim_right());
                self.writeln(&format!("{:—<1$}", "", textwrap::termwidth()), Color::Red, true);
                self.fail_count += 1;
                self.scenarios.insert(scenario.clone(), ScenarioResult::Fail);
            },
            TestResult::MutexPoisoned => {
                self.writeln_cmt(&format!("- {}", msg), cmt, indent, Color::Cyan, false);
                self.print_step_extras(step);

                self.write("    ⚡ ", Color::Yellow, false);
                println!("Skipped due to previous error (poisoned)");
                self.fail_count += 1;
            },
            TestResult::Skipped => {
                self.writeln_cmt(&format!("- {}", msg), cmt, indent, Color::Cyan, false);
                self.print_step_extras(step);
                self.skipped_count += 1;
            }
            TestResult::Unimplemented => {
                self.writeln_cmt(&format!("- {}", msg), cmt, indent, Color::Cyan, false);
                self.print_step_extras(step);
                self.write("    ⚡ ", Color::Yellow, false);
                println!("Not yet implemented (skipped)");
                self.skipped_count += 1;
            }
        };
    }

    fn visit_finish(&mut self) {
        self.print_finish().unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct HashableRegex(pub Regex);

impl Hash for HashableRegex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}

impl PartialEq for HashableRegex {
    fn eq(&self, other: &HashableRegex) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for HashableRegex {}

impl Deref for HashableRegex {
    type Target = Regex;

    fn deref(&self) -> &Regex {
        &self.0
    }
}

type TestFn<T> = fn(&mut T, &Step) -> ();
type TestRegexFn<T> = fn(&mut T, &[String], &Step) -> ();

pub struct TestCase<T: Default> {
    pub test: TestFn<T>
}

impl<T: Default> TestCase<T> {
    #[allow(dead_code)]
    pub fn new(test: TestFn<T>) -> TestCase<T> {
        TestCase {
            test: test
        }
    }
}

pub struct RegexTestCase<'a, T: 'a + Default> {
    pub test: TestRegexFn<T>,
    _marker: std::marker::PhantomData<&'a T>
}

impl<'a, T: Default> RegexTestCase<'a, T> {
    #[allow(dead_code)]
    pub fn new(test: TestRegexFn<T>) -> RegexTestCase<'a, T> {
        RegexTestCase {
            test: test,
            _marker: std::marker::PhantomData
        }
    }
}

pub struct Steps<'s, T: 's + Default> {
    pub given: HashMap<&'static str, TestCase<T>>,
    pub when: HashMap<&'static str, TestCase<T>>,
    pub then: HashMap<&'static str, TestCase<T>>,
    pub regex: RegexSteps<'s, T>
}

pub struct RegexSteps<'s, T: 's + Default> {
    pub given: HashMap<HashableRegex, RegexTestCase<'s, T>>,
    pub when: HashMap<HashableRegex, RegexTestCase<'s, T>>,
    pub then: HashMap<HashableRegex, RegexTestCase<'s, T>>,
}

pub enum TestCaseType<'a, T> where T: 'a, T: Default {
    Normal(&'a TestCase<T>),
    Regex(&'a RegexTestCase<'a, T>, Vec<String>)
}

pub enum TestResult {
    MutexPoisoned,
    Skipped,
    Unimplemented,
    Pass,
    Fail(String, String)
}

struct Sink(Arc<Mutex<Vec<u8>>>);
impl Write for Sink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        Write::write(&mut *self.0.lock().unwrap(), data)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct CapturedIo<T> {
    stdout: Vec<u8>,
    result: Result<T, Box<dyn Any + Send>>
}

fn capture_io<T, F: FnOnce() -> T>(callback: F) -> CapturedIo<T> {
    let data = Arc::new(Mutex::new(Vec::new()));
    let data2 = data.clone();

    let old_io = (
        io::set_print(Some(Box::new(Sink(data2.clone())))),
        io::set_panic(Some(Box::new(Sink(data2))))
    );

    let result = panic::catch_unwind(
        panic::AssertUnwindSafe(callback)
    );

    let captured_io = CapturedIo {
        stdout: data.lock().unwrap().to_vec(),
        result: result
    };

    io::set_print(old_io.0);
    io::set_panic(old_io.1);

    captured_io
}


impl<'s, T: Default> Steps<'s, T> {
    #[allow(dead_code)]
    pub fn new() -> Steps<'s, T> {
        let regex_tests = RegexSteps {
            given: HashMap::new(),
            when: HashMap::new(),
            then: HashMap::new()
        };

        let tests = Steps {
            given: HashMap::new(),
            when: HashMap::new(),
            then: HashMap::new(),
            regex: regex_tests
        };

        tests
    }

    fn test_bag_for<'a>(&self, ty: StepType) -> &HashMap<&'static str, TestCase<T>> {
        match ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then
        }
    }

    fn regex_bag_for<'a>(&'a self, ty: StepType) -> &HashMap<HashableRegex, RegexTestCase<'a, T>> {
        match ty {
            StepType::Given => &self.regex.given,
            StepType::When => &self.regex.when,
            StepType::Then => &self.regex.then
        }
    }

    fn test_type(&'s self, step: &Step) -> Option<TestCaseType<'s, T>> {
        let test_bag = self.test_bag_for(step.ty);

        match test_bag.get(&*step.value) {
            Some(v) => Some(TestCaseType::Normal(v)),
            None => {
                let regex_bag = self.regex_bag_for(step.ty);

                let result = regex_bag.iter()
                    .find(|(regex, _)| regex.is_match(&step.value));

                match result {
                    Some((regex, tc)) => {
                        let matches = regex.0.captures(&step.value).unwrap();
                        let matches: Vec<String> = matches.iter().map(|x| x.unwrap().as_str().to_string()).collect();
                        Some(TestCaseType::Regex(tc, matches))
                    },
                    None => {
                        None
                    }
                }
            }
        }
    }

    fn run_test_inner<'a>(
        &'s self,
        world: &mut T,
        test_type: TestCaseType<'s, T>,
        step: &'a gherkin::Step
    ) {
        match test_type {
            TestCaseType::Normal(t) => (t.test)(world, &step),
            TestCaseType::Regex(t, ref c) => (t.test)(world, c, &step)
        };
    }

    fn run_test<'a>(&'s self, world: &mut T, test_type: TestCaseType<'s, T>, step: &'a Step, last_panic: Arc<Mutex<Option<String>>>) -> TestResult {
        let last_panic_hook = last_panic.clone();
        panic::set_hook(Box::new(move |info| {
            let mut state = last_panic.lock().expect("last_panic unpoisoned");
            *state = info.location().map(|x| format!("{}:{}:{}", x.file(), x.line(), x.column()));
        }));


        let captured_io = capture_io(move || {
            self.run_test_inner(world, test_type, &step)
        });

        let _ = panic::take_hook();
        
        match captured_io.result {
            Ok(_) => TestResult::Pass,
            Err(any) => {
                let mut state = last_panic_hook.lock().expect("unpoisoned");
                let loc = match &*state {
                    Some(v) => &v,
                    None => "unknown"
                };

                let s = {
                    if let Some(s) = any.downcast_ref::<String>() {
                        s.as_str()
                    } else if let Some(s) = any.downcast_ref::<&str>() {
                        *s
                    } else {
                        ""
                    }
                };

                if s == "not yet implemented" {
                    TestResult::Unimplemented
                } else {
                    let panic_str = if &captured_io.stdout.len() > &0usize {
                        String::from_utf8_lossy(&captured_io.stdout).to_string()
                    } else {
                        format!("Panicked with: {}", s)
                    };
                    TestResult::Fail(panic_str, loc.to_owned())
                }
            }
        }
    }

    fn run_scenario<'a>(
        &'s self,
        feature: &'a gherkin::Feature,
        scenario: &'a gherkin::Scenario,
        last_panic: Arc<Mutex<Option<String>>>,
        output: &mut impl OutputVisitor
    ) {
        output.visit_scenario(&scenario);

        let captured_io = capture_io(|| T::default());
        let mut world = match captured_io.result {
            Ok(v) => v,
            Err(e) => {
                if &captured_io.stdout.len() > &0usize {
                    let msg = String::from_utf8_lossy(&captured_io.stdout).to_string();
                    panic!(msg);
                } else {
                    panic!(e);
                }
            }
        };
        
        let mut steps: Vec<&'a Step> = vec![];
        if let Some(ref bg) = &feature.background {
            for s in &bg.steps {
                steps.push(&s);
            }
        }

        for s in &scenario.steps {
            steps.push(&s);
        }

        let mut is_skipping = false;

        for step in steps.iter() {
            output.visit_step(&scenario, &step);

            let test_type = match self.test_type(&step) {
                Some(v) => v,
                None => {
                    output.visit_step_result(&scenario, &step, &TestResult::Unimplemented);
                    if !is_skipping {
                        is_skipping = true;
                        output.visit_scenario_skipped(&scenario);
                    }
                    continue;
                }
            };

            if is_skipping {
                output.visit_step_result(&scenario, &step, &TestResult::Skipped);
            } else {
                let result = self.run_test(&mut world, test_type, &step, last_panic.clone());
                output.visit_step_result(&scenario, &step, &result);
                match result {
                    TestResult::Pass => {}
                    _ => {
                        is_skipping = true;
                        output.visit_scenario_skipped(&scenario);
                    }
                };
            }
        }

        output.visit_scenario_end(&scenario);
    }
    
    pub fn run<'a>(&'s self, feature_path: &Path, output: &mut impl OutputVisitor) {
        output.visit_start();
        
        let feature_path = fs::read_dir(feature_path).expect("feature path to exist");
        let last_panic: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        for entry in feature_path {
            let path = entry.unwrap().path();
            let mut file = File::open(&path).expect("file to open");
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).unwrap();
            
            let feature = Feature::from(&*buffer);
            output.visit_feature(&feature, &path);

            for scenario in (&feature.scenarios).iter() {
                self.run_scenario(&feature, &scenario, last_panic.clone(), output);
            }

            output.visit_feature_end(&feature);
        }
        
        output.visit_finish();
    }
}

#[macro_export]
macro_rules! cucumber {
    (
        features: $featurepath:tt;
        world: $worldtype:path;
        steps: $vec:expr;
        before: $beforefn:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; Some(Box::new($beforefn)));
    };

    (
        features: $featurepath:tt;
        world: $worldtype:path;
        steps: $vec:expr
    ) => {
        cucumber!(@finish; $featurepath; $worldtype; $vec; None);
    };

    (
        @finish; $featurepath:tt; $worldtype:path; $vec:expr; $beforefn:expr
    ) => {
        #[allow(unused_imports)]
        fn main() {
            use std::path::Path;
            use std::process;
            use std::boxed::FnBox;
            use $crate::{Steps, World, DefaultOutput};

            let path = match Path::new($featurepath).canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}", e);
                    eprintln!("There was an error parsing \"{}\"; aborting.", $featurepath);
                    process::exit(1);
                }
            };

            if !&path.exists() {
                eprintln!("Path {:?} does not exist; aborting.", &path);
                process::exit(1);
            }

            let tests = {
                let step_groups: Vec<Steps<$worldtype>> = $vec.iter().map(|f| f()).collect();
                let mut combined_steps = Steps::new();

                for step_group in step_groups.into_iter() {
                    combined_steps.given.extend(step_group.given);
                    combined_steps.when.extend(step_group.when);
                    combined_steps.then.extend(step_group.then);

                    combined_steps.regex.given.extend(step_group.regex.given);
                    combined_steps.regex.when.extend(step_group.regex.when);
                    combined_steps.regex.then.extend(step_group.regex.then);
                }

                combined_steps
            };
            
            let mut output = DefaultOutput::default();

            let before_fn: Option<Box<FnBox() -> ()>> = $beforefn;

            match before_fn {
                Some(f) => f(),
                None => {}
            };

            tests.run(&path, &mut output);
        }
    }
}

#[macro_export]
macro_rules! steps {
    (
        @gather_steps, $tests:tt,
        $ty:ident regex $name:tt $body:expr;
    ) => {
        $tests.regex.$ty.insert(
            HashableRegex(Regex::new($name).expect(&format!("{} is a valid regex", $name))),
                RegexTestCase::new($body));
    };

    (
        @gather_steps, $tests:tt,
        $ty:ident regex $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.regex.$ty.insert(
            HashableRegex(Regex::new($name).expect(&format!("{} is a valid regex", $name))),
                RegexTestCase::new($body));

        steps!(@gather_steps, $tests, $( $items )*);
    };

    (
        @gather_steps, $tests:tt,
        $ty:ident $name:tt $body:expr;
    ) => {
        $tests.$ty.insert($name, TestCase::new($body));
    };

    (
        @gather_steps, $tests:tt,
        $ty:ident $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.$ty.insert($name, TestCase::new($body));

        steps!(@gather_steps, $tests, $( $items )*);
    };

    (
        world: $worldtype:path;
        $( $items:tt )*
    ) => {
        #[allow(unused_imports)]
        pub fn steps<'a>() -> $crate::Steps<'a, $worldtype> {
            use std::path::Path;
            use std::process;
            use $crate::regex::Regex;
            use $crate::{Steps, TestCase, RegexTestCase, HashableRegex};

            let mut tests: Steps<'a, $worldtype> = Steps::new();
            steps!(@gather_steps, tests, $( $items )*);
            tests
        }
    };
}


#[cfg(test)]
mod tests {
    use std::default::Default;

    pub struct World {
        pub thing: bool
    }

    impl ::World for World {}

    impl Default for World {
        fn default() -> World {
            World {
                thing: false
            }
        }
    }
}

#[cfg(test)]
mod tests1 {
    steps! {
        world: ::tests::World;
        when regex "^test (.*) regex$" |_world, matches, _step| {
            println!("{}", matches[1]);
        };

        given "a thing" |_world, _step| {
            assert!(true);
        };

        when "another thing" |_world, _step| {
            assert!(false);
        };

        when "something goes right" |_world, _step| { 
            assert!(true);
        };

        then "another thing" |_world, _step| {
            assert!(true)
        };

        when "nothing" |world, step| {
            panic!("oh shit");
        };
    }
}

#[cfg(test)]
cucumber! {
    features: "./features";
    world: tests::World;
    steps: &[
        tests1::steps
    ]
}