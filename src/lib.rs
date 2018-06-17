pub extern crate gherkin;
pub extern crate regex;

use gherkin::{Step, StepType, Feature};
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::ops::Deref;
use std::panic;
use std::path::Path;
use std::sync::Mutex;

pub struct HashableRegex(Regex);

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

pub struct TestCase<T: Default> {
    pub test: fn(&mut T) -> ()
}

impl<T: Default> TestCase<T> {
    #[allow(dead_code)]
    pub fn new(test: fn(&mut T) -> ()) -> TestCase<T> {
        TestCase {
            test: test
        }
    }
}

pub struct RegexTestCase<T: Default> {
    pub test: fn(&mut T, &[String]) -> ()
}

impl<T: Default> RegexTestCase<T> {
    #[allow(dead_code)]
    pub fn new(test: fn(&mut T, &[String]) -> ()) -> RegexTestCase<T> {
        RegexTestCase {
            test: test
        }
    }
}

pub struct CucumberTests<T: Default> {
    pub given: HashMap<&'static str, TestCase<T>>,
    pub when: HashMap<&'static str, TestCase<T>>,
    pub then: HashMap<&'static str, TestCase<T>>,
    pub regex: CucumberRegexTests<T>
}

pub struct CucumberRegexTests<T: Default> {
    pub given: HashMap<HashableRegex, RegexTestCase<T>>,
    pub when: HashMap<HashableRegex, RegexTestCase<T>>,
    pub then: HashMap<HashableRegex, RegexTestCase<T>>,
}

enum TestType<'a, T> where T: 'a, T: Default {
    Normal(&'a TestCase<T>),
    Regex(&'a RegexTestCase<T>, Vec<String>)
}

impl<T: Default> CucumberTests<T> {
    #[allow(dead_code)]
    pub fn new() -> CucumberTests<T> {
        let regex_tests = CucumberRegexTests {
            given: HashMap::new(),
            when: HashMap::new(),
            then: HashMap::new()
        };

        let tests = CucumberTests {
            given: HashMap::new(),
            when: HashMap::new(),
            then: HashMap::new(),
            regex: regex_tests
        };

        tests
    }

    #[allow(dead_code)]
    fn test_type<'a>(&'a self, step: &Step, value: &str) -> Option<TestType<'a, T>> {
        let test_bag = match step.ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then
        };

        match test_bag.get(value) {
            Some(v) => Some(TestType::Normal(v)),
            None => {
                let regex_bag = match step.ty {
                    StepType::Given => &self.regex.given,
                    StepType::When => &self.regex.when,
                    StepType::Then => &self.regex.then
                };

                let result = regex_bag.iter()
                    .find(|(regex, _)| regex.is_match(&value));

                match result {
                    Some((regex, tc)) => {
                        let thing = regex.0.captures(&value).unwrap();
                        let matches: Vec<String> = thing.iter().map(|x| x.unwrap().as_str().to_string()).collect();
                        Some(TestType::Regex(tc, matches))
                    },
                    None => {
                        None
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn run(&mut self, feature_path: &Path) {
        use std::sync::Arc;

        let last_panic: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let last_panic_hook = last_panic.clone();

        panic::set_hook(Box::new(move |info| {
            let mut state = last_panic_hook.lock().expect("last_panic unpoisoned");
            *state = info.location().map(|x| format!("{}:{}:{}", x.file(), x.line(), x.column()));
        }));

        let feature_path = fs::read_dir(feature_path).expect("feature path to exist");

        let mut scenarios = 0;
        let mut steps = 0;

        for entry in feature_path {
            let mut file = File::open(entry.unwrap().path()).expect("file to open");
            let mut buffer = String::new();
            file.read_to_string(&mut buffer).unwrap();
            let feature = Feature::from(&*buffer);
            
            println!("Feature: {}\n", feature.name);

            for scenario in feature.scenarios {
                scenarios += 1;

                println!("  Scenario: {}", scenario.name);

                let mut world = Mutex::new(T::default());

                for step in scenario.steps {
                    steps += 1;

                    let value = step.value.to_string();
                    
                    let test_type = match self.test_type(&step, &value) {
                        Some(v) => v,
                        None => {
                            println!("    {}\n      # No test found", &step.to_string());
                            continue;
                        }
                    };

                    let result = panic::catch_unwind(|| {
                        match world.lock() {
                            Ok(mut world) => {
                                match test_type {
                                    TestType::Normal(t) => (t.test)(&mut *world),
                                    TestType::Regex(t, c) => (t.test)(&mut *world, &c)
                                }
                            },
                            Err(e) => {
                                return Err(e);
                            }
                        };

                        return Ok(())
                    });

                    println!("    {:<40}", &step.to_string());

                    match result {
                        Ok(inner) => {
                            match inner {
                                Ok(_) => {},
                                Err(_) => println!("      # Skipped due to previous error")
                            }
                        }
                        Err(any) => {
                            println!("      # Step failed:");
                            let mut state = last_panic.lock().expect("unpoisoned");

                            {
                                let loc = match &*state {
                                    Some(v) => &v,
                                    None => "unknown"
                                };

                                if let Some(s) = any.downcast_ref::<String>() {
                                    println!("      # {}  [{}]", &s, loc);
                                } else if let Some(s) = any.downcast_ref::<&str>() {
                                    println!("      # {}  [{}]", &s, loc);
                                }
                            }

                            *state = None;
                        }
                    };
                }

                println!("");
            }
        }

        let _ = panic::take_hook();

        println!("# Scenarios: {}", scenarios);
        println!("# Steps: {}", steps);
    }
}

#[macro_export]
macro_rules! cucumber {
    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt $body:expr;
    ) => {
        $tests.regex.$ty.insert(
            HashableRegex(Regex::new($name).expect(&format!("{} is a valid regex", $name))),
            RegexTestCase::new($body));
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident regex $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.regex.$ty.insert(
            HashableRegex(Regex::new($name).expect(&format!("{} is a valid regex", $name))),
            RegexTestCase::new($body));

        cucumber!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident $name:tt $body:expr;
    ) => {
        $tests.$ty.insert($name, TestCase::new($body));
    };

    (
        @gather_steps, $worldtype:path, $tests:tt,
        $ty:ident $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.$ty.insert($name, TestCase::new($body));

        cucumber!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        features: $featurepath:tt;
        world: $worldtype:path;
        $( $items:tt )*
    ) => {
        #[allow(unused_imports)]
        fn main() {
            use std::path::Path;
            use $crate::regex::Regex;
            use $crate::{CucumberTests, TestCase, RegexTestCase, HashableRegex};

            let mut tests: CucumberTests<$worldtype> = CucumberTests::new();
            cucumber!(@gather_steps, $worldtype, tests, $( $items )*);
            tests.run(Path::new($featurepath));
        }
    };
}


#[cfg(test)]
mod tests {
    use std::default::Default;

    pub struct World {
        pub thing: bool
    }

    impl Default for World {
        fn default() -> World {
            World {
                thing: false
            }
        }
    }
}

#[cfg(test)]
cucumber! {
    features: "./features";
    world: tests::World;

    when regex "^test (.*) regex$" |world, matches| {
        println!("{}", matches[1]);
    };

    given "a thing" |world| {
        assert!(true);
    };

    when "another thing" |world| {
        assert!(true);
    };

    when "something goes right" |world| { 
        assert!(true);
    };

    then "another thing" |world| {
        assert!(true)
    };
}