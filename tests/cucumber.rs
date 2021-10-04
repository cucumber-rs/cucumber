use std::{convert::Infallible, time::Duration};

use async_trait::async_trait;
use cucumber::{given, then, when, World, WorldInit};
use tokio::time::sleep;

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
async fn hungry_cat(world: &mut AnimalWorld, state: String) {
    sleep(Duration::from_secs(2)).await;

    match state.as_str() {
        "hungry" => world.cat.hungry = true,
        "satiated" => world.cat.hungry = false,
        _ => unreachable!(),
    }
}

#[when("I feed the cat")]
async fn feed_cat(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    world.cat.feed();
}

#[then("the cat is not hungry")]
async fn cat_is_fed(world: &mut AnimalWorld) {
    sleep(Duration::from_secs(2)).await;

    assert!(!world.cat.hungry);
}

#[tokio::main]
async fn main() {
    AnimalWorld::cucumber()
        .fail_on_skipped_with(|_, _, sc| !sc.tags.iter().any(|t| t == "dog"))
        .run_and_exit("tests/features/asciinema.feature")
        .await;
}
