extern crate cucumber_rust as cucumber;
use async_trait::async_trait;

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
    bar: usize,
}

impl MyWorld {
    async fn test_async_fn(&mut self) -> Option<usize> {
        Some(123890)
    }
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    async fn new() -> Self {
        Self { foo: "wat".into(), bar: 0 }
    }
}

mod example_steps {
    use cucumber::Steps;
    use futures::future::FutureExt;
    use std::rc::Rc;

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given(
                "a thing",
                Rc::new(|mut world, _step| {
                    async move {
                        world.foo = "elho".into();
                        world.bar = world.test_async_fn().await.unwrap();
                        world
                    }
                    .catch_unwind()
                    .boxed_local()
                }),
            )
            .when_regex(
                "something goes (.*)",
                Rc::new(|world, _matches, _step| async move { world }.catch_unwind().boxed_local()),
            )
            .given_sync(
                "I am trying out Cucumber",
                |mut world: crate::MyWorld, _step| {
                    world.foo = "Some string".to_string();
                    world
                },
            )
            .when_sync("I consider what I am doing", |mut world, _step| {
                let new_string = format!("{}.", &world.foo);
                world.foo = new_string;
                world
            })
            .then_sync("I am interested in ATDD", |world, _step| {
                assert_eq!(world.foo, "Some string.");
                world
            })
            .then_regex_sync(
                r"^we can (.*) rules with regex$",
                |world, matches, _step| {
                    // And access them as an array
                    assert_eq!(matches[1], "implement");
                    world
                },
            );

        builder
    }
}

fn main() {
    // Do any setup you need to do before running the Cucumber runner.
    // e.g. setup_some_db_thing()?;

    let runner = cucumber::Cucumber::<MyWorld>::new()
        .features(&["./features"])
        .steps(example_steps::steps());

    // You may choose any executor you like (Tokio, async-std, etc)
    // You may even have an async main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(runner.run());
}
