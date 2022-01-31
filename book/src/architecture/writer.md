Custom `Writer`
===============

Finally, let's implement a custom [`Writer`] which simply outputs [cucumber events][`event::Cucumber`] to [STDOUT] in the order of receiving.

[`Writer`] represents anything that consumes a [`Stream`] of [cucumber events][`event::Cucumber`].

```rust
# use std::{
#     convert::Infallible,
#     panic::{self, AssertUnwindSafe},
#     path::PathBuf,
#     sync::Arc,
#     time::Duration,
# };
#
# use async_trait::async_trait;
# use cucumber::{
#     cli, event, gherkin, given, parser, step, then, when, Event, World,
#     WorldInit, WriterExt as _,
# };
# use futures::{
#     future::{self, FutureExt as _},
#     stream::{self, LocalBoxStream, Stream, StreamExt as _, TryStreamExt as _},
# };
# use once_cell::sync::Lazy;
# use tokio::time::sleep;
#
# #[derive(Clone, Copy, Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Clone, Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
# struct CustomParser;
#
# impl<I> cucumber::Parser<I> for CustomParser {
#     type Cli = cli::Empty;
#     type Output = stream::Once<future::Ready<parser::Result<gherkin::Feature>>>;
#
#     fn parse(self, _: I, _: Self::Cli) -> Self::Output {
#         let keyword = "Feature";
#         let name = "Animal feature";
#         stream::once(future::ok(gherkin::Feature {
#             keyword: keyword.into(),
#             name: name.into(),
#             description: None,
#             background: None,
#             scenarios: vec![gherkin::Scenario {
#                 keyword: "Scenario".into(),
#                 name: "If we feed a hungry cat it won't be hungry".into(),
#                 steps: vec![
#                     gherkin::Step {
#                         keyword: "Given".into(),
#                         ty: gherkin::StepType::Given,
#                         value: "a hungry cat".into(),
#                         docstring: None,
#                         table: None,
#                         span: gherkin::Span { start: 5, end: 18 },
#                         position: gherkin::LineCol { line: 3, col: 5 },
#                     },
#                     gherkin::Step {
#                         keyword: "When".into(),
#                         ty: gherkin::StepType::When,
#                         value: "I feed the cat".into(),
#                         docstring: None,
#                         table: None,
#                         span: gherkin::Span { start: 5, end: 19 },
#                         position: gherkin::LineCol { line: 4, col: 5 },
#                     },
#                     gherkin::Step {
#                         keyword: "Then".into(),
#                         ty: gherkin::StepType::Then,
#                         value: "the cat is not hungry".into(),
#                         docstring: None,
#                         table: None,
#                         span: gherkin::Span { start: 5, end: 26 },
#                         position: gherkin::LineCol { line: 5, col: 5 },
#                     },
#                 ],
#                 examples: vec![],
#                 tags: vec![],
#                 span: gherkin::Span { start: 3, end: 52 },
#                 position: gherkin::LineCol { line: 2, col: 3 },
#             }],
#             rules: vec![],
#             tags: vec![],
#             span: gherkin::Span { start: 1, end: 23 },
#             position: gherkin::LineCol { line: 1, col: 1 },
#             path: Some(PathBuf::from(file!())),
#         }))
#     }
# }
#
# struct CustomRunner;
#
# impl CustomRunner {
#     fn steps_fns() -> &'static step::Collection<AnimalWorld> {
#         static STEPS: Lazy<step::Collection<AnimalWorld>> =
#             Lazy::new(AnimalWorld::collection);
#         &STEPS
#     }
#
#     async fn execute_step(
#         mut world: AnimalWorld,
#         step: gherkin::Step,
#     ) -> (AnimalWorld, event::Step<AnimalWorld>) {
#         let ev = if let Some((step_fn, captures, ctx)) =
#             Self::steps_fns().find(&step).expect("Ambiguous match")
#         {
#             match AssertUnwindSafe(step_fn(&mut world, ctx))
#                 .catch_unwind()
#                 .await
#             {
#                 Ok(()) => event::Step::Passed(captures),
#                 Err(e) => event::Step::Failed(
#                     Some(captures),
#                     Some(Arc::new(world.clone())),
#                     event::StepError::Panic(e.into()),
#                 ),
#             }
#         } else {
#             event::Step::Skipped
#         };
#         (world, ev)
#     }
#
#     async fn execute_scenario(
#         scenario: gherkin::Scenario,
#     ) -> impl Stream<Item = event::Feature<AnimalWorld>> {
#         let hook = panic::take_hook();
#         panic::set_hook(Box::new(|_| {}));
#
#         let mut world = AnimalWorld::new().await.unwrap();
#         let mut steps = Vec::with_capacity(scenario.steps.len());
#
#         for step in scenario.steps.clone() {
#             let (w, ev) = Self::execute_step(world, step.clone()).await;
#             world = w;
#             let should_stop = matches!(ev, event::Step::Failed(..));
#             steps.push((step, ev));
#             if should_stop {
#                 break;
#             }
#         }
#
#         panic::set_hook(hook);
#
#         let scenario = Arc::new(scenario);
#         stream::once(future::ready(event::Scenario::Started))
#             .chain(stream::iter(steps.into_iter().flat_map(|(step, ev)| {
#                 let step = Arc::new(step);
#                 [
#                     event::Scenario::Step(step.clone(), event::Step::Started),
#                     event::Scenario::Step(step, ev),
#                 ]
#             })))
#             .chain(stream::once(future::ready(event::Scenario::Finished)))
#             .map(move |ev| event::Feature::Scenario(scenario.clone(), ev))
#     }
#
#     fn execute_feature(
#         feature: gherkin::Feature,
#     ) -> impl Stream<Item = event::Cucumber<AnimalWorld>> {
#         let feature = Arc::new(feature);
#         stream::once(future::ready(event::Feature::Started))
#             .chain(
#                 stream::iter(feature.scenarios.clone())
#                     .then(Self::execute_scenario)
#                     .flatten(),
#             )
#             .chain(stream::once(future::ready(event::Feature::Finished)))
#             .map(move |ev| event::Cucumber::Feature(feature.clone(), ev))
#     }
# }
#
# impl cucumber::Runner<AnimalWorld> for CustomRunner {
#     type Cli = cli::Empty;
#     type EventStream = LocalBoxStream<
#         'static,
#         parser::Result<Event<event::Cucumber<AnimalWorld>>>,
#     >;
#
#     fn run<S>(self, features: S, _: Self::Cli) -> Self::EventStream
#     where
#         S: Stream<Item = parser::Result<gherkin::Feature>> + 'static,
#     {
#         stream::once(future::ok(event::Cucumber::Started))
#             .chain(
#                 features
#                     .map_ok(|f| Self::execute_feature(f).map(Ok))
#                     .try_flatten(),
#             )
#             .chain(stream::once(future::ok(event::Cucumber::Finished)))
#             .map_ok(Event::new)
#             .boxed_local()
#     }
# }
#
struct CustomWriter;

#[async_trait(?Send)]
impl<W: 'static> cucumber::Writer<W> for CustomWriter {
    type Cli = cli::Empty; // we provide no CLI options

    async fn handle_event(
        &mut self,
        ev: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        match ev {
            Ok(Event { value, .. }) => match value {
                event::Cucumber::Feature(feature, ev) => match ev {
                    event::Feature::Started => {
                        println!("{}: {}", feature.keyword, feature.name)
                    }
                    event::Feature::Scenario(scenario, ev) => match ev {
                        event::Scenario::Started => {
                            println!("{}: {}", scenario.keyword, scenario.name)
                        }
                        event::Scenario::Step(step, ev) => match ev {
                            event::Step::Started => {
                                print!("{} {}...", step.keyword, step.value)
                            }
                            event::Step::Passed(_) => println!("ok"),
                            event::Step::Skipped => println!("skip"),
                            event::Step::Failed(_, _, err) => {
                                println!("failed: {err}")
                            }
                        },
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            Err(e) => println!("Error: {e}"),
        }
    }
}

#[tokio::main]
async fn main() {
    AnimalWorld::cucumber::<&str>() // aiding type inference for `CustomParser`
        .with_parser(CustomParser)
        .with_runner(CustomRunner)
        .with_writer(CustomWriter.assert_normalized()) // OK because of `CustomRunner`
        .run("tests/features/book")
        .await;
}
```
![record](../rec/architecture_writer_raw.gif)

> __TIP__: `CustomWriter` will print trash if we feed unordered [`event::Cucumber`]s into it. Though, we shouldn't care about order normalization in our implementations. Instead, we may just wrap `CustomWriter` into [`writer::Normalize`], which will do that for us.

```rust
# use std::{convert::Infallible, path::PathBuf, time::Duration};
#
# use async_trait::async_trait;
# use cucumber::{
#     cli, event, gherkin, given, parser, then, when, Event, World, WorldInit,
#     WriterExt as _,
# };
# use futures::{future, stream};
# use tokio::time::sleep;
#
# #[derive(Clone, Copy, Debug, Default)]
# struct Animal {
#     pub hungry: bool,
# }
#
# impl Animal {
#     fn feed(&mut self) {
#         self.hungry = false;
#     }
# }
#
# #[derive(Clone, Debug, WorldInit)]
# pub struct AnimalWorld {
#     cat: Animal,
# }
#
# #[async_trait(?Send)]
# impl World for AnimalWorld {
#     type Error = Infallible;
#
#     async fn new() -> Result<Self, Infallible> {
#         Ok(Self {
#             cat: Animal::default(),
#         })
#     }
# }
#
# #[given(regex = r"^a (hungry|satiated) cat$")]
# async fn hungry_cat(world: &mut AnimalWorld, state: String) {
#     sleep(Duration::from_secs(2)).await;
#
#     match state.as_str() {
#         "hungry" => world.cat.hungry = true,
#         "satiated" => world.cat.hungry = false,
#         _ => unreachable!(),
#     }
# }
#
# #[when("I feed the cat")]
# async fn feed_cat(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     world.cat.feed();
# }
#
# #[then("the cat is not hungry")]
# async fn cat_is_fed(world: &mut AnimalWorld) {
#     sleep(Duration::from_secs(2)).await;
#
#     assert!(!world.cat.hungry);
# }
#
# struct CustomParser;
#
# impl<I> cucumber::Parser<I> for CustomParser {
#     type Cli = cli::Empty;
#     type Output = stream::Once<future::Ready<parser::Result<gherkin::Feature>>>;
#
#     fn parse(self, _: I, _: Self::Cli) -> Self::Output {
#         let keyword = "Feature";
#         let name = "Animal feature";
#         stream::once(future::ok(gherkin::Feature {
#             keyword: keyword.into(),
#             name: name.into(),
#             description: None,
#             background: None,
#             scenarios: vec![gherkin::Scenario {
#                 keyword: "Scenario".into(),
#                 name: "If we feed a hungry cat it won't be hungry".into(),
#                 steps: vec![
#                     gherkin::Step {
#                         keyword: "Given".into(),
#                         ty: gherkin::StepType::Given,
#                         value: "a hungry cat".into(),
#                         docstring: None,
#                         table: None,
#                         span: gherkin::Span { start: 5, end: 18 },
#                         position: gherkin::LineCol { line: 3, col: 5 },
#                     },
#                     gherkin::Step {
#                         keyword: "When".into(),
#                         ty: gherkin::StepType::When,
#                         value: "I feed the cat".into(),
#                         docstring: None,
#                         table: None,
#                         span: gherkin::Span { start: 5, end: 19 },
#                         position: gherkin::LineCol { line: 4, col: 5 },
#                     },
#                     gherkin::Step {
#                         keyword: "Then".into(),
#                         ty: gherkin::StepType::Then,
#                         value: "the cat is not hungry".into(),
#                         docstring: None,
#                         table: None,
#                         span: gherkin::Span { start: 5, end: 26 },
#                         position: gherkin::LineCol { line: 5, col: 5 },
#                     },
#                 ],
#                 examples: vec![],
#                 tags: vec![],
#                 span: gherkin::Span { start: 3, end: 52 },
#                 position: gherkin::LineCol { line: 2, col: 3 },
#             }],
#             rules: vec![],
#             tags: vec![],
#             span: gherkin::Span { start: 1, end: 23 },
#             position: gherkin::LineCol { line: 1, col: 1 },
#             path: Some(PathBuf::from(file!())),
#         }))
#     }
# }
#
# struct CustomWriter;
#
# #[async_trait(?Send)]
# impl<W: 'static> cucumber::Writer<W> for CustomWriter {
#     type Cli = cli::Empty; // we provide no CLI options
#
#     async fn handle_event(
#         &mut self,
#         ev: parser::Result<Event<event::Cucumber<W>>>,
#         _: &Self::Cli,
#     ) {
#         match ev {
#             Ok(Event { value, .. }) => match value {
#                 event::Cucumber::Feature(feature, ev) => match ev {
#                     event::Feature::Started => {
#                         println!("{}: {}", feature.keyword, feature.name)
#                     }
#                     event::Feature::Scenario(scenario, ev) => match ev {
#                         event::Scenario::Started => {
#                             println!("{}: {}", scenario.keyword, scenario.name)
#                         }
#                         event::Scenario::Step(step, ev) => match ev {
#                             event::Step::Started => {
#                                 print!("{} {}...", step.keyword, step.value)
#                             }
#                             event::Step::Passed(_) => println!("ok"),
#                             event::Step::Skipped => println!("skip"),
#                             event::Step::Failed(_, _, err) => {
#                                 println!("failed: {err}", )
#                             }
#                         },
#                         _ => {}
#                     },
#                     _ => {}
#                 },
#                 _ => {}
#             },
#             Err(e) => println!("Error: {e}"),
#         }
#     }
# }
#
#[tokio::main]
async fn main() {
    AnimalWorld::cucumber::<&str>() // aiding type inference for `CustomParser`
        .with_parser(CustomParser)
        .with_writer(CustomWriter.normalized()) // wrapping into `writer::Normalize`,
        .run("tests/features/book")             // so it works OK with the default
        .await;                                 // concurrent `Runner`
}
```
![record](../rec/architecture_writer_normalized.gif)

> __NOTE__: [`Writer`]s are easily pipelined. See [`WriterExt`] trait and [`writer`] module for more [`Writer`] machinery "included batteries".




[`event::Cucumber`]: https://docs.rs/cucumber/*/cucumber/event/enum.Cucumber.html
[`Stream`]: https://docs.rs/futures/*/futures/stream/trait.Stream.html
[`writer`]: https://docs.rs/cucumber/*/cucumber/writer/index.html
[`Writer`]: https://docs.rs/cucumber/*/cucumber/trait.Writer.html
[`WriterExt`]: https://docs.rs/cucumber/*/cucumber/trait.WriterExt.html
[`writer::Normalize`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Normalize.html
[STDOUT]: https://en.wikipedia.org/wiki/Standard_streams#Standard_output_(stdout)
