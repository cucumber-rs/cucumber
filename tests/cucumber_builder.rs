extern crate cucumber_rust as cucumber;
use cucumber::{after, before, CucumberBuilder};

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
}

impl cucumber::World for MyWorld {}
impl std::default::Default for MyWorld {
    fn default() -> MyWorld {
        // This function is called every time a new scenario is started
        MyWorld {
            foo: "a default string".to_string(),
        }
    }
}

mod example_steps {
    use cucumber::{typed_regex, Steps, StepsBuilder};

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: StepsBuilder<crate::MyWorld> = StepsBuilder::new();

        builder
            .given("a thing", |_world, _step| {

            })
            .when_regex("something goes (.*)", typed_regex!(crate::MyWorld, (String) |world, item, _step| {
                world.foo = item;
            }))
            .given("I am trying out Cucumber", |world, _step| {
                world.foo = "Some string".to_string();
            })
            .when("I consider what I am doing", |world, _step| {
                let new_string = format!("{}.", &world.foo);
                world.foo = new_string;
            })
            .then("I am interested in ATDD", |world, _step| {
                assert_eq!(world.foo, "Some string.");
            })
            .then_regex(
                r"^we can (.*) rules with regex$",
                |_world, matches, _step| {
                    // And access them as an array
                    assert_eq!(matches[1], "implement");
                },
            )
            .then_regex(
                r"^we can also match (\d+) (.+) types$",
                typed_regex!(
                    crate::MyWorld,
                    (usize, String) | _world,
                    num,
                    word,
                    _step | {
                        // `num` will be of type usize, `word` of type String
                        assert_eq!(num, 42);
                        assert_eq!(word, "olika");
                    }
                ),
            );

        builder.build()
    }
}

// Declares a before handler function named `a_before_fn`
before!(a_before_fn => |_scenario| {

});

// Declares an after handler function named `an_after_fn`
after!(an_after_fn => |_scenario| {

});

// A setup function to be called before everything else
fn setup() {}

fn main() {
    let mut builder = CucumberBuilder::new(cucumber::DefaultOutput::default());

    builder
        .features(vec!["./features".into()])
        .setup(setup)
        .steps(example_steps::steps());

    builder.command_line();
}
