use std::convert::Infallible;

use async_trait::async_trait;
use cucumber::{given, then, when, World, WorldInit};

#[derive(Debug)]
struct Cat {
    pub hungry: bool,
}

impl Cat {
    fn feed(&mut self) {
        self.hungry = false;
    }
}

#[derive(Debug, WorldInit)]
pub struct AnimalWorld {
    cat: Cat,
}

#[async_trait(?Send)]
impl World for AnimalWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            cat: Cat { hungry: false },
        })
    }
}

#[given(regex = r"^a (hungry|satiated) cat$")]
fn hungry_cat(world: &mut AnimalWorld, state: String) {
    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when("I feed the cat")]
fn feed_cat(world: &mut AnimalWorld) {
    world.cat.feed();
}

#[then("the cat is not hungry")]
fn cat_is_fed(world: &mut AnimalWorld) {
    assert!(!world.cat.hungry);
}

fn main() {
    futures::executor::block_on(
        AnimalWorld::cucumber().fail_on_skipped().run_and_exit(
            "/tests/features/book/getting_started/concurrent.feature",
        ),
    );
}
