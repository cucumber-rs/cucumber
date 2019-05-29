#[macro_use]
extern crate cucumber_rust;

use std::default::Default;

pub struct MyWorld {
    pub thing: bool
}

impl cucumber_rust::World for MyWorld {}

impl Default for MyWorld {
    fn default() -> MyWorld {
        MyWorld {
            thing: false
        }
    }
}

#[cfg(test)]
mod basic {
    steps!(::MyWorld => {
        when regex "thing (\\d+)" (usize) |_world, _sz, _step| {

        };
        
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

        when "something goes wrong" |_world, _step| {
            println!("Something to stdout");
            eprintln!("Something to stderr");
            panic!("This is my custom panic message");
        };

        then "another thing" |_world, _step| {
            assert!(true)
        };
    });
}

fn before_thing(_step: &cucumber_rust::Scenario) {

}

before!(some_before: "@tag2 and @tag3" => |_scenario| {
    println!("{}", "lol");
});

before!(something_great => |_scenario| {

});

after!(after_thing => |_scenario| {

});

fn setup() {

}

cucumber! {
    features: "./features",
    world: ::MyWorld,
    steps: &[
        basic::steps
    ],
    setup: setup,
    before: &[before_thing, some_before, something_great],
    after: &[after_thing]
}
