//! Execution methods for Cucumber executor.

use std::mem;

use futures::{StreamExt as _};

use crate::{
    Parser, Runner, World, Writer, cli, writer,
    tag::Ext as _,
};

use super::core::Cucumber;

impl<W, P, I, R, Wr, Cli> Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: Writer<W> + writer::Normalized,
    Cli: clap::Args,
{
    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// [`Feature`]: gherkin::Feature
    pub async fn run(self, input: I) -> Wr {
        self.filter_run(input, |_, _, _| true).await
    }

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// # Example
    ///
    /// Adjust [`Cucumber`] to run only [`Scenario`]s marked with `@cat` tag:
    /// ```rust
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .filter_run("tests/features/readme", |_, _, sc| {
    ///         sc.tags.iter().any(|t| t == "cat")
    ///     })
    ///     .await;
    /// # }
    /// ```
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   @cat
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   @dog
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    /// <script
    ///     id="asciicast-0KvTxnfaMRjsvsIKsalS611Ta"
    ///     src="https://asciinema.org/a/0KvTxnfaMRjsvsIKsalS611Ta.js"
    ///     async data-autoplay="true" data-rows="14">
    /// </script>
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    pub async fn filter_run<F>(self, input: I, filter: F) -> Wr
    where
        F: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        let cli::Opts {
            re_filter,
            tags_filter,
            parser: parser_cli,
            runner: runner_cli,
            writer: writer_cli,
            ..
        } = self.cli.unwrap_or_else(cli::Opts::<_, _, _, _>::parsed);

        let filter = move |feat: &gherkin::Feature,
                           rule: Option<&gherkin::Rule>,
                           scenario: &gherkin::Scenario| {
            re_filter.as_ref().map_or_else(
                || {
                    tags_filter.as_ref().map_or_else(
                        || filter(feat, rule, scenario),
                        |tags| {
                            // The order `Feature` -> `Rule` -> `Scenario`
                            // matters here.
                            tags.eval(
                                feat.tags
                                    .iter()
                                    .chain(rule.iter().flat_map(|r| &r.tags))
                                    .chain(scenario.tags.iter()),
                            )
                        },
                    )
                },
                |re| re.is_match(&scenario.name),
            )
        };

        let Self { parser, runner, mut writer, .. } = self;

        let features = parser.parse(input, parser_cli);

        let filtered = features.map(move |feature| {
            let mut feature = feature?;
            let feat_scenarios = mem::take(&mut feature.scenarios);
            feature.scenarios = feat_scenarios
                .into_iter()
                .filter(|s| filter(&feature, None, s))
                .collect();

            let mut rules = mem::take(&mut feature.rules);
            for r in &mut rules {
                let rule_scenarios = mem::take(&mut r.scenarios);
                r.scenarios = rule_scenarios
                    .into_iter()
                    .filter(|s| filter(&feature, Some(r), s))
                    .collect();
            }
            feature.rules = rules;

            Ok(feature)
        });

        let events_stream = runner.run(filtered, runner_cli);
        futures::pin_mut!(events_stream);
        while let Some(ev) = events_stream.next().await {
            writer.handle_event(ev, &writer_cli).await;
        }
        writer
    }
}

impl<W, I, P, R, Wr, Cli> Cucumber<W, P, I, R, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    R: Runner<W>,
    Wr: writer::Stats<W> + writer::Normalized,
    Cli: clap::Args,
{
    /// Runs [`Cucumber`].
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] [`Failed`].
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Feature`]: gherkin::Feature
    /// [`Step`]: gherkin::Step
    pub async fn run_and_exit(self, input: I) {
        self.filter_run_and_exit(input, |_, _, _| true).await;
    }

    /// Runs [`Cucumber`] with [`Scenario`]s filter.
    ///
    /// [`Feature`]s sourced from a [`Parser`] are fed to a [`Runner`], which
    /// produces events handled by a [`Writer`].
    ///
    /// # Panics
    ///
    /// If encountered errors while parsing [`Feature`]s or at least one
    /// [`Step`] [`Failed`].
    ///
    /// # Example
    ///
    /// Adjust [`Cucumber`] to run only [`Scenario`]s marked with `@cat` tag:
    /// ```rust
    /// # use cucumber::World;
    /// #
    /// # #[derive(Debug, Default, World)]
    /// # struct MyWorld;
    /// #
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// MyWorld::cucumber()
    ///     .filter_run_and_exit("tests/features/readme", |_, _, sc| {
    ///         sc.tags.iter().any(|t| t == "cat")
    ///     })
    ///     .await;
    /// # }
    /// ```
    /// ```gherkin
    /// Feature: Animal feature
    ///
    ///   @cat
    ///   Scenario: If we feed a hungry cat it will no longer be hungry
    ///     Given a hungry cat
    ///     When I feed the cat
    ///     Then the cat is not hungry
    ///
    ///   @dog
    ///   Scenario: If we feed a satiated dog it will not become hungry
    ///     Given a satiated dog
    ///     When I feed the dog
    ///     Then the dog is not hungry
    /// ```
    /// <script
    ///     id="asciicast-0KvTxnfaMRjsvsIKsalS611Ta"
    ///     src="https://asciinema.org/a/0KvTxnfaMRjsvsIKsalS611Ta.js"
    ///     async data-autoplay="true" data-rows="14">
    /// </script>
    ///
    /// [`Failed`]: event::Step::Failed
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    pub async fn filter_run_and_exit<Filter>(self, input: I, filter: Filter)
    where
        Filter: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> bool
            + 'static,
    {
        let writer = self.filter_run(input, filter).await;
        if writer.execution_has_failed() {
            let mut msg = Vec::with_capacity(3);

            let failed_steps = writer.failed_steps();
            if failed_steps > 0 {
                msg.push(format!(
                    "{failed_steps} step{} failed",
                    if failed_steps > 1 { "s" } else { "" },
                ));
            }

            let parsing_errors = writer.parsing_errors();
            if parsing_errors > 0 {
                msg.push(format!(
                    "{parsing_errors} parsing error{}",
                    if parsing_errors > 1 { "s" } else { "" },
                ));
            }

            let hook_errors = writer.hook_errors();
            if hook_errors > 0 {
                msg.push(format!(
                    "{hook_errors} hook error{}",
                    if hook_errors > 1 { "s" } else { "" },
                ));
            }

            panic!("{}", msg.join(", "));
        }
    }
}