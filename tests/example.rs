use std::{convert::Infallible, time::Duration};

use async_trait::async_trait;
use cucumber_rust::{self as cucumber, given, then, when, WorldInit};
use tokio::time::sleep;

#[derive(Debug, WorldInit)]
struct World {
    user: Option<String>,
    capacity: usize,
}

#[async_trait(? Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            user: None,
            capacity: 0,
        })
    }
}

#[given(regex = r"^(\S+) is hungry$")]
async fn someone_is_hungry(w: &mut World, user: String) {
    sleep(Duration::from_secs(2)).await;

    w.user = Some(user);
}

#[when(regex = r"^(?:he|she|they) eats? (\d+) cucumbers?$")]
async fn eat_cucumbers(w: &mut World, count: usize) {
    sleep(Duration::from_secs(2)).await;

    w.capacity += count;

    if w.capacity > 3 {
        panic!("{} exploded!", w.user.as_ref().unwrap());
    }
}

#[then(regex = r"^(?:he|she|they) (?:is|are) full$")]
async fn is_full(w: &mut World) {
    sleep(Duration::from_secs(2)).await;

    assert_eq!(w.capacity, 3, "{} isn't full!", w.user.as_ref().unwrap(),);
}

#[tokio::main]
async fn main() {
    World::run("tests/features/example").await;
}
