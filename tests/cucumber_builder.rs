extern crate cucumber_rust as cucumber;

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
}

#[async_trait::async_trait(?Send)]
impl cucumber::World for MyWorld {
    // async fn new() -> Self {
    //     todo!()
    // }
    async fn new() -> Self {
        Self { foo: "wat".into() }
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
                        panic!("UUUUUU");
                        world.foo = "elho".into();
                        world
                    }
                    .catch_unwind()
                    .boxed_local()
                }),
            )
            // .when_regex(
            //     "something goes (.*)",
            //     typed_regex!(
            //         crate::MyWorld,
            //         (String) | world,
            //         item,
            //         _step | {
            //             world.foo = item;
            //         }
            //     ),
            // )
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
            });
        // .then_regex(
        //     r"^we can (.*) rules with regex$",
        //     |_world, matches, _step| {
        //         // And access them as an array
        //         assert_eq!(matches[1], "implement");
        //     },
        // )
        // .then_regex(
        //     r"^we can also match (\d+) (.+) types$",
        //     typed_regex!(
        //         crate::MyWorld,
        //         (usize, String) | _world,
        //         num,
        //         word,
        //         _step | {
        //             // `num` will be of type usize, `word` of type String
        //             assert_eq!(num, 42);
        //             assert_eq!(word, "olika");
        //         }
        //     ),
        // );

        builder
    }
}

// // Declares a before handler function named `a_before_fn`
// before!(a_before_fn => |_scenario| {

// });

// // Declares an after handler function named `an_after_fn`
// after!(an_after_fn => |_scenario| {

// });

// // A setup function to be called before everything else
// fn setup() {}

fn main() {
    let m = cucumber::Cucumber::<MyWorld>::new()
        .features(&["./features"])
        .steps(example_steps::steps());
    // let mut builder = CucumberBuilder::new(cucumber::DefaultOutput::default());

    // builder
    //     .features(vec!["./features".into()])
    //     .setup(setup)
    //     .steps(example_steps::steps());

    futures::executor::block_on(m.run());
}
