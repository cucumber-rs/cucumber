//! Repeat functionality for Cucumber executor.

use std::marker::PhantomData;

use crate::{
    Parser, Runner, World, Writer, WriterExt as _, event, parser, writer,
    Event,
};

use super::core::Cucumber;

impl<W, P, I, R, Wr, Cli> Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W>,
    Cli: clap::Args,
{
    /// Re-outputs [`Skipped`] steps for easier navigation.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::run()`]:
    /// <script
    ///     id="asciicast-0d92qlT8Mbc4WXyvRbHJmjsqN"
    ///     src="https://asciinema.org/a/0d92qlT8Mbc4WXyvRbHJmjsqN.js"
    ///     async data-autoplay="true" data-rows="17">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all the [`Skipped`] steps at the end:
    /// ```rust
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .repeat_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-ox14HynkBIw8atpfhyfvKrsO3"
    ///     src="https://asciinema.org/a/ox14HynkBIw8atpfhyfvKrsO3.js"
    ///     async data-autoplay="true" data-rows="19">
    /// </script>
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [`Skipped`]: event::Step::Skipped
    #[must_use]
    pub fn repeat_skipped(
        self,
    ) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr>, Cli>
    where
        Wr: writer::NonTransforming,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_skipped(),
            cli: self.cli,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Re-outputs [`Failed`] steps for easier navigation.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::fail_on_skipped()`]:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-UcipuopO6IFEsIDty6vaJlCH9"
    ///     src="https://asciinema.org/a/UcipuopO6IFEsIDty6vaJlCH9.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all the [`Failed`] steps at the end:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .repeat_failed()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-ofOljvyEMb41OTLhE081QKv68"
    ///     src="https://asciinema.org/a/ofOljvyEMb41OTLhE081QKv68.js"
    ///     async data-autoplay="true" data-rows="24">
    /// </script>
    ///
    /// [`Failed`]: event::Step::Failed
    #[must_use]
    pub fn repeat_failed(
        self,
    ) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr>, Cli>
    where
        Wr: writer::NonTransforming,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_failed(),
            cli: self.cli,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }

    /// Re-outputs steps by the given `filter` predicate.
    ///
    /// # Example
    ///
    /// Output with a regular [`Cucumber::fail_on_skipped()`]:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// # use futures::FutureExt as _;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-UcipuopO6IFEsIDty6vaJlCH9"
    ///     src="https://asciinema.org/a/UcipuopO6IFEsIDty6vaJlCH9.js"
    ///     async data-autoplay="true" data-rows="21">
    /// </script>
    ///
    /// Adjust [`Cucumber`] to re-output all the [`Failed`] steps ta the end by
    /// providing a custom `filter` predicate:
    /// ```rust,should_panic
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .repeat_if(|ev| {
    ///         use cucumber::event::{
    ///             Cucumber, Feature, RetryableScenario, Rule, Scenario, Step,
    ///         };
    ///
    ///         matches!(
    ///             ev.as_deref(),
    ///             Ok(Cucumber::Feature(
    ///                 _,
    ///                 Feature::Rule(
    ///                     _,
    ///                     Rule::Scenario(
    ///                         _,
    ///                         RetryableScenario {
    ///                             event: Scenario::Step(_, Step::Failed(..))
    ///                                 | Scenario::Background(
    ///                                     _,
    ///                                     Step::Failed(_, _, _, _),
    ///                                 ),
    ///                             retries: _
    ///                         }
    ///                     )
    ///                 ) | Feature::Scenario(
    ///                     _,
    ///                     RetryableScenario {
    ///                         event: Scenario::Step(_, Step::Failed(..))
    ///                             | Scenario::Background(_, Step::Failed(..)),
    ///                         retries: _
    ///                     }
    ///                 )
    ///             )) | Err(_)
    ///         )
    ///     })
    ///     .fail_on_skipped()
    ///     .run_and_exit("tests/features/readme")
    ///     .await;
    /// # }
    /// ```
    /// <script
    ///     id="asciicast-ofOljvyEMb41OTLhE081QKv68"
    ///     src="https://asciinema.org/a/ofOljvyEMb41OTLhE081QKv68.js"
    ///     async data-autoplay="true" data-rows="24">
    /// </script>
    ///
    /// [`Failed`]: event::Step::Failed
    #[must_use]
    pub fn repeat_if<F>(
        self,
        filter: F,
    ) -> Cucumber<W, P, I, R, writer::Repeat<W, Wr, F>, Cli>
    where
        F: Fn(&parser::Result<Event<event::Cucumber<W>>>) -> bool,
        Wr: writer::NonTransforming,
    {
        Cucumber {
            parser: self.parser,
            runner: self.runner,
            writer: self.writer.repeat_if(filter),
            cli: self.cli,
            _world: PhantomData,
            _parser_input: PhantomData,
        }
    }
}