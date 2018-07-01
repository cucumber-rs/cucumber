// Copyright (c) 2018  Brendan Molloy <brendan@bbqsrc.net>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(set_stdio)]

pub extern crate gherkin_rust as gherkin;
pub extern crate regex;
extern crate termcolor;

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

pub trait World: Default {}

pub trait OutputVisitor : Default {
    fn visit_start(&mut self);
    fn visit_feature(&mut self, feature: &gherkin::Feature);
    fn visit_feature_end(&mut self, feature: &gherkin::Feature);
    fn visit_scenario(&mut self, scenario: &gherkin::Scenario);
    fn visit_scenario_end(&mut self, scenario: &gherkin::Scenario);
    fn visit_step(&mut self, step: &gherkin::Step);
    fn visit_step_result(&mut self, step: &gherkin::Step, result: &TestResult);
    fn visit_finish(&mut self);
}

pub struct DefaultOutput {
    stdout: StandardStream,
    scenario_count: u32,
    scenario_fail_count: u32,
    step_count: u32,
    skipped_count: u32,
    fail_count: u32
}

impl std::default::Default for DefaultOutput {
    fn default() -> DefaultOutput {
        DefaultOutput {
            stdout: StandardStream::stdout(ColorChoice::Always),
            scenario_count: 0,
            scenario_fail_count: 0,
            step_count: 0,
            skipped_count: 0,
            fail_count: 0
        }
    }
}

impl DefaultOutput {
    fn write(&mut self, s: &str, c: Color, bold: bool) {
        self.stdout.set_color(ColorSpec::new().set_fg(Some(c)).set_bold(bold)).unwrap();
        writeln!(&mut self.stdout, "{}", s).unwrap();
        self.stdout.set_color(ColorSpec::new().set_fg(None)).unwrap();
    }

    fn red(&mut self, s: &str) {
        self.write(s, Color::Red, false);
    }

    fn green(&mut self, s: &str) {
        self.write(s, Color::Green, false);
    }

    fn bold_green(&mut self, s: &str) {
        self.write(s, Color::Green, true);
    }

    fn cyan(&mut self, s: &str) {
        self.write(s, Color::Cyan, false);
    }
}

impl OutputVisitor for DefaultOutput {
    fn visit_start(&mut self) {
        self.bold_green(&format!("[Cucumber v{}]\n", env!("CARGO_PKG_VERSION")))
    }
    
    fn visit_feature(&mut self, feature: &gherkin::Feature) {
        self.bold_green(&format!("Feature: {}\n", &feature.name));
    }
    
    fn visit_feature_end(&mut self, _feature: &gherkin::Feature) {}

    fn visit_scenario(&mut self, scenario: &gherkin::Scenario) {
        self.bold_green(&format!("  Scenario: {}", &scenario.name));
        self.scenario_count += 1;
    }
    
    fn visit_scenario_end(&mut self, _scenario: &gherkin::Scenario) {
        println!("");
    }
    
    fn visit_step(&mut self, _step: &gherkin::Step) {
        self.step_count += 1;
    }
    
    fn visit_step_result(&mut self, step: &gherkin::Step, result: &TestResult) {
        match result {
            TestResult::Pass => {
                self.green(&format!("    {:<60}", &step.to_string()));
                if let Some(ref docstring) = &step.docstring {
                    println!("      \"\"\"\n      {}\n      \"\"\"", docstring);
                }
            },
            TestResult::Fail(msg, loc) => {
                self.red(&format!("    {:<60}", &step.to_string()));
                self.red(&format!("{}  [{}]", msg, loc));
                self.fail_count += 1;
                self.scenario_fail_count += 1;
            },
            TestResult::MutexPoisoned => {
                self.cyan(&format!("    {:<60}", &step.to_string()));
                self.cyan("      # Skipped due to previous error (poisoned)");
                self.fail_count += 1;
            },
            TestResult::Skipped => {
                self.cyan(&format!("    {:<60}", &step.to_string()));
                self.skipped_count += 1;
            }
            TestResult::Unimplemented => {
                self.cyan(&format!("    {:<60}", &step.to_string()));
                self.cyan("      # Not yet implemented (skipped)");
                self.skipped_count += 1;
            }
        };
    }

    fn visit_finish(&mut self) {
        self.stdout.set_color(ColorSpec::new()
            .set_fg(Some(Color::Green))
            .set_bold(true)).unwrap();
            
        // Do scenario count
        write!(&mut self.stdout, "{} scenarios", &self.scenario_count).unwrap();
        if self.scenario_fail_count > 0 {
            write!(&mut self.stdout, " ({} failed)", &self.scenario_fail_count).unwrap();
        }
        println!();

        // Do steps
        let mut o = vec![];
        if self.fail_count > 0 {
            o.push(format!("{} failed", self.fail_count));
        }
        if self.skipped_count > 0 {
            o.push(format!("{} skipped", self.skipped_count));
        }
        let passed_count = self.step_count - self.skipped_count - self.fail_count;
        o.push(format!("{} passed", passed_count));

        let msg = format!("{} steps ({})", &self.step_count, o.join(", "));
        writeln!(&mut self.stdout, "{}\n", &msg).unwrap();
        
        self.stdout.set_color(ColorSpec::new()
            .set_fg(None)
            .set_bold(false)).unwrap();
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

fn capture_io<T, F: FnOnce() -> T>(callback: F) -> Result<T, Box<dyn Any + Send>>{
    let data = Arc::new(Mutex::new(Vec::new()));
    let data2 = data.clone();

    let old_io = (
        io::set_print(Some(Box::new(Sink(data2.clone())))),
        io::set_panic(Some(Box::new(Sink(data2))))
    );

    let result = panic::catch_unwind(
        panic::AssertUnwindSafe(callback)
    );

    io::set_print(old_io.0);
    io::set_panic(old_io.1);

    result
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


        let result = capture_io(move || {
            self.run_test_inner(world, test_type, &step)
        });

        let _ = panic::take_hook();
        
        match result {
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
                    TestResult::Fail(s.to_owned(), loc.to_owned())
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

        let mut world = match capture_io(|| T::default()) {
            Ok(v) => v,
            Err(e) => {
                panic!(e);
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
            output.visit_step(&step);

            let test_type = match self.test_type(&step) {
                Some(v) => v,
                None => {
                    output.visit_step_result(&step, &TestResult::Unimplemented);
                    continue;
                }
            };

            if is_skipping {
                output.visit_step_result(&step, &TestResult::Skipped);
            } else {
                let result = self.run_test(&mut world, test_type, &step, last_panic.clone());
                output.visit_step_result(&step, &result);
                match result {
                    TestResult::Pass => {}
                    _ => { is_skipping = true; }
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
            let mut file = File::open(entry.unwrap().path()).expect("file to open");
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).unwrap();
            
            let feature = Feature::from(&*buffer);
            output.visit_feature(&feature);

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
        steps: $vec:expr
    ) => {
        #[allow(unused_imports)]
        fn main() {
            use std::path::Path;
            use std::process;
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