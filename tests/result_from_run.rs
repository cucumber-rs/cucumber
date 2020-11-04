extern crate cucumber_rust as cucumber;
use async_trait::async_trait;
use std::convert::Infallible;

pub struct MyWorld;

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

mod example_steps {
    use cucumber::Steps;

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given("nothing", |world: crate::MyWorld, _step| world)
            .given("a panic", |_world, _step| panic!("Expected panic step"));

        builder
    }
}

fn main() {
    let runner = cucumber::Cucumber::<MyWorld>::new()
        .features(&["./features/failing"])
        .steps(example_steps::steps());

    futures::executor::block_on(runner.run()).unwrap_err();
}
