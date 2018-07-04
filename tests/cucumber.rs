#![feature(fnbox)]

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
    steps! {
        world: ::MyWorld;
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
            // panic!("oh shit");
        };
    }
}

cucumber! {
    features: "./features";
    world: ::MyWorld;
    steps: &[
        basic::steps
    ]
}
