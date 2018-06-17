pub extern crate gherkin;

macro_rules! cucumber {
    (
        @gather_steps, $worldtype:ident, $tests:tt,
        $ty:ident $name:tt $body:expr;
    ) => {
        $tests.$ty.insert($name, $body);
    };

    (
        @gather_steps, $worldtype:ident, $tests:tt,
        $ty:ident $name:tt $body:expr; $( $items:tt )*
    ) => {
        $tests.$ty.insert($name, $body);

        cucumber!(@gather_steps, $worldtype, $tests, $( $items )*);
    };

    (
        $featurepath:tt; $worldtype:ident; $( $items:tt )*
    ) => {
        mod cucumber_tests {
            use super::$worldtype;
            use std::fs::{self, File};
            use std::collections::HashMap;
            use $crate::gherkin::{Feature, StepType};
            use std::io::prelude::*;
            use std::panic;
            use std::sync::Mutex;

            struct CucumberTests {
                pub given: HashMap<&'static str, fn(&mut $worldtype) -> ()>,
                pub when: HashMap<&'static str, fn(&mut $worldtype) -> ()>,
                pub then: HashMap<&'static str, fn(&mut $worldtype) -> ()>
            }

            impl CucumberTests {
                #[allow(unused_assignments)]
                #[allow(unused_mut)]
                fn new() -> CucumberTests {
                    let mut given = HashMap::new();
                    let mut when = HashMap::new();
                    let mut then = HashMap::new();

                    let mut tests = CucumberTests {
                        given: given,
                        when: when,
                        then: then
                    };

                    cucumber!(@gather_steps, $worldtype, tests, $( $items )*);

                    tests
                }

                pub fn run(self) {
                    let feature_path = fs::read_dir($featurepath).expect("feature path to exist");

                    for entry in feature_path {
                        let mut file = File::open(entry.unwrap().path()).expect("file to open");
                        let mut buffer = String::new();
                        file.read_to_string(&mut buffer).unwrap();
                        let feature = Feature::from(&*buffer);
                        
                        println!("Feature: {}", feature.name);

                        for scenario in feature.scenarios {
                            println!("  Scenario: {}", scenario.name);

                            let mut world = Mutex::new($worldtype::default());

                            for step in scenario.steps {
                                let test_bag = match step.ty {
                                    StepType::Given => &self.given,
                                    StepType::When => &self.when,
                                    StepType::Then => &self.then
                                };

                                let value = &*step.value;

                                let test = match test_bag.get(value) {
                                    Some(v) => v,
                                    None => {
                                        println!("    {}: NO TEST FOUND", value);
                                        continue;
                                    }
                                };

                                print!("    {}: ", value);
                                let result = panic::catch_unwind(|| {
                                    let mut world = world.lock().expect("world unpoisoned");
                                    test(&mut *world);
                                });

                                match result {
                                    Ok(_) => println!("PASS"),
                                    Err(_) => {
                                        println!("FAIL; ending scenario.");
                                        break;
                                    }
                                };
                            }
                        }
                    }
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

cucumber! { "../features"; World;

given "a thing" |world| {
    // println!("given a thing was run!");
};

when "another thing" |world| {
    assert!(false);
};

when "something goes right" |world| { 
    // println!("{}", world.thing);
    let foo = 42;
};

then "another thing" |world| {
    assert!(true)
};

}