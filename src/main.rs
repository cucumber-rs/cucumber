pub extern crate gherkin;
extern crate regex;

macro_rules! cucumber {
    (
        @gather_steps, $worldtype:ident, $tests:tt,
        $ty:ident regex $name:tt $body:expr;
    ) => {
        $tests.regex.$ty.insert(
            HashableRegex(Regex::new($name).expect(&format!("{} is a valid regex", $name))),
            RegexTestCase::new($body));
    };

    (
        @gather_steps, $worldtype:ident, $tests:tt,
        $ty:ident regex $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.regex.$ty.insert(
            HashableRegex(Regex::new($name).expect(&format!("{} is a valid regex", $name))),
            RegexTestCase::new($body));

        cucumber!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        @gather_steps, $worldtype:ident, $tests:tt,
        $ty:ident $name:tt $body:expr;
    ) => {
        $tests.$ty.insert($name, TestCase::new($body));
    };

    (
        @gather_steps, $worldtype:ident, $tests:tt,
        $ty:ident $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.$ty.insert($name, TestCase::new($body));

        cucumber!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        $featurepath:tt; $worldtype:ident; $( $items:tt )*
    ) => {
        mod cucumber_tests {
            use regex::{Regex, Captures};
            use super::$worldtype;
            use std::fs::{self, File};
            use std::collections::HashMap;
            use $crate::gherkin::{Feature, StepType, Step};
            use std::io::prelude::*;
            use std::panic;
            use std::sync::Mutex;

            struct TestCase {
                pub test: fn(&mut $worldtype) -> ()
            }

            impl TestCase {
                fn new(test: fn(&mut $worldtype) -> ()) -> TestCase {
                    TestCase {
                        test: test
                    }
                }
            }

            struct RegexTestCase {
                pub test: fn(&mut $worldtype, &[String]) -> ()
            }

            impl RegexTestCase {
                fn new(test: fn(&mut $worldtype, &[String]) -> ()) -> RegexTestCase {
                    RegexTestCase {
                        test: test
                    }
                }
            }

            use std::hash::{Hash, Hasher};
            use std::ops::Deref;

            struct HashableRegex(Regex);
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

            struct CucumberTests {
                pub given: HashMap<&'static str, TestCase>,
                pub when: HashMap<&'static str, TestCase>,
                pub then: HashMap<&'static str, TestCase>,
                pub regex: CucumberRegexTests
            }

            struct CucumberRegexTests {
                pub given: HashMap<HashableRegex, RegexTestCase>,
                pub when: HashMap<HashableRegex, RegexTestCase>,
                pub then: HashMap<HashableRegex, RegexTestCase>,
            }

            enum TestType<'a> {
                Normal(&'a TestCase),
                Regex(&'a RegexTestCase, Vec<String>)
            }

            impl CucumberTests {
                #[allow(unused_assignments)]
                #[allow(unused_mut)]
                fn new() -> CucumberTests {
                    let mut given = HashMap::new();
                    let mut when = HashMap::new();
                    let mut then = HashMap::new();

                    let mut given_regex = HashMap::new();
                    let mut when_regex = HashMap::new();
                    let mut then_regex = HashMap::new();

                    let mut regex_tests = CucumberRegexTests {
                        given: given_regex,
                        when: when_regex,
                        then: then_regex
                    };

                    let mut tests = CucumberTests {
                        given: given,
                        when: when,
                        then: then,
                        regex: regex_tests
                    };

                    cucumber!(@gather_steps, $worldtype, tests, $( $items )*);

                    tests
                }

                fn test_type<'a>(&'a self, step: &Step, value: &str) -> Option<TestType<'a>> {
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

                pub fn run(&mut self) {
                    use std::sync::Arc;

                    let last_panic: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
                    let last_panic_hook = last_panic.clone();

                    panic::set_hook(Box::new(move |info| {
                        // println!("{:?}", info.location());
                        let mut state = last_panic_hook.lock().expect("last_panic unpoisoned");
                        *state = info.location().map(|x| format!("{}:{}:{}", x.file(), x.line(), x.column()));
                    }));

                    let feature_path = fs::read_dir($featurepath).expect("feature path to exist");

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

                            let mut world = Mutex::new($worldtype::default());

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

                                        // break;
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

            pub fn run() {
                CucumberTests::new().run()
            }
        }
        
        fn main() {
            cucumber_tests::run();
        }
    };

}

use std::default::Default;

struct World {
    thing: bool
}

impl Default for World {
    fn default() -> World {
        World {
            thing: false
        }
    }
}

cucumber! { "./features"; World;

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