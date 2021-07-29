use std::{cell::RefCell, convert::Infallible};

use async_trait::async_trait;
use cucumber_rust::{given, then, when, World, WorldInit, WorldRun as _};

#[derive(Debug, WorldInit)]
pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
    bar: usize,
    some_value: RefCell<u8>,
}

impl MyWorld {
    async fn test_async_fn(&mut self) {
        *self.some_value.borrow_mut() = 123_u8;
        self.bar = 123;
    }
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            foo: "wat".into(),
            bar: 0,
            some_value: RefCell::new(0),
        })
    }
}

#[given("a thing")]
async fn a_thing(world: &mut MyWorld) {
    world.foo = "elho".into();
    world.test_async_fn().await;
}

#[when(regex = "something goes (.*)")]
fn something_goes(_: &mut MyWorld, _wrong: String) {}

#[given("I am trying out Cucumber")]
fn i_am_trying_out(world: &mut MyWorld) {
    world.foo = "Some string".to_string();
}

#[when("I consider what I am doing")]
fn i_consider(world: &mut MyWorld) {
    let new_string = format!("{}.", &world.foo);
    world.foo = new_string;
}

#[then("I am interested in ATDD")]
fn i_am_interested(world: &mut MyWorld) {
    assert_eq!(world.foo, "Some string.");
}

#[then(regex = r"^we can (.*) rules with regex$")]
fn we_can_regex(_: &mut MyWorld, action: String) {
    // `action` can be anything implementing `FromStr`.
    assert_eq!(action, "implement");
}

fn main() {
    let runner = MyWorld::run("./features");
    futures::executor::block_on(runner);
}
