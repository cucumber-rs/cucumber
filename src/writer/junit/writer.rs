//! Main JUnit XML writer implementation.

use std::{fmt::Debug, io, time::SystemTime};

use junit_report::Report;

use crate::{
    Event, World, Writer, event, parser,
    writer::{self, Ext as _, Verbosity, discard},
};

use super::{
    cli::Cli,
    event_handlers::EventHandler,
    test_case_builder::JUnitTestCaseBuilder,
};

/// [JUnit XML report][1] [`Writer`] implementation outputting XML to an
/// [`io::Write`] implementor.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into
/// a [`writer::Normalize`], otherwise will panic in runtime as won't be able to
/// form correct [JUnit `testsuite`s][1].
///
/// [`Normalized`]: writer::Normalized
/// [1]: https://llg.cubic.org/docs/junit
#[derive(Debug)]
pub struct JUnit<W, Out: io::Write> {
    /// [`io::Write`] implementor to output XML report into.
    output: Out,

    /// [JUnit XML report][1].
    ///
    /// [1]: https://llg.cubic.org/docs/junit
    report: Report,

    /// Current [JUnit `testsuite`][1].
    ///
    /// [1]: https://llg.cubic.org/docs/junit
    suit: Option<junit_report::TestSuite>,

    /// [`SystemTime`] when the current [`Scenario`] has started.
    ///
    /// [`Scenario`]: gherkin::Scenario
    scenario_started_at: Option<SystemTime>,

    /// Current [`Scenario`] [events][1].
    ///
    /// [`Scenario`]: gherkin::Scenario
    /// [1]: event::Scenario
    events: Vec<event::RetryableScenario<W>>,

    /// Event handler for processing different event types.
    event_handler: EventHandler<W, Out>,

    /// [`Verbosity`] of this [`Writer`].
    verbosity: Verbosity,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World: std::fmt::Debug + crate::World, Out: Clone + io::Write> Clone for JUnit<World, Out> {
    fn clone(&self) -> Self {
        Self {
            output: self.output.clone(),
            report: self.report.clone(),
            suit: self.suit.clone(),
            scenario_started_at: self.scenario_started_at,
            events: self.events.clone(),
            event_handler: EventHandler::<World, Out>::new(
                JUnitTestCaseBuilder::new(self.verbosity)
            ),
            verbosity: self.verbosity,
        }
    }
}

impl<W, Out> Writer<W> for JUnit<W, Out>
where
    W: World + Debug,
    Out: io::Write,
{
    type Cli = Cli;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        use event::{Cucumber, Feature, Rule};

        self.apply_cli(*cli);

        match event.map(Event::split) {
            Err(err) => {
                EventHandler::<W, Out>::handle_parser_error(&mut self.report, &err);
            }
            Ok((Cucumber::Started | Cucumber::ParsingFinished { .. }, _)) => {}
            Ok((Cucumber::Feature(feat, ev), meta)) => match ev {
                Feature::Started => {
                    self.suit = Some(EventHandler::<W, Out>::handle_feature_started(&feat, meta));
                }
                Feature::Rule(_, Rule::Started | Rule::Finished) => {}
                Feature::Rule(r, Rule::Scenario(sc, ev)) => {
                    self.event_handler.handle_scenario_event(
                        &feat,
                        Some(&r),
                        &sc,
                        ev,
                        meta,
                        &mut self.scenario_started_at,
                        &mut self.events,
                        &mut self.suit,
                    );
                }
                Feature::Scenario(sc, ev) => {
                    self.event_handler.handle_scenario_event(
                        &feat,
                        None,
                        &sc,
                        ev,
                        meta,
                        &mut self.scenario_started_at,
                        &mut self.events,
                        &mut self.suit,
                    );
                }
                Feature::Finished => {
                    let suite = EventHandler::<W, Out>::handle_feature_finished(&feat, self.suit.take());
                    self.report.add_testsuite(suite);
                }
            },
            Ok((Cucumber::Finished, _)) => {
                EventHandler::<W, Out>::handle_cucumber_finished(&mut self.report, &mut self.output);
            }
        }
    }
}

impl<W, O: io::Write> writer::NonTransforming for JUnit<W, O> {}

impl<W: Debug + World, Out: io::Write> JUnit<W, Out> {
    /// Creates a new [`Normalized`] [`JUnit`] [`Writer`] outputting XML report
    /// into the given `output`.
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn new(
        output: Out,
        verbosity: impl Into<Verbosity>,
    ) -> writer::Normalize<W, Self> {
        Self::raw(output, verbosity).normalized()
    }

    /// Creates a new non-[`Normalized`] [`JUnit`] [`Writer`] outputting XML
    /// report into the given `output`, and suitable for feeding into [`tee()`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [`tee()`]: crate::WriterExt::tee
    /// [1]: https://llg.cubic.org/docs/junit
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub fn for_tee(
        output: Out,
        verbosity: impl Into<Verbosity>,
    ) -> discard::Arbitrary<discard::Stats<Self>> {
        Self::raw(output, verbosity)
            .discard_stats_writes()
            .discard_arbitrary_writes()
    }

    /// Creates a new raw and non-[`Normalized`] [`JUnit`] [`Writer`] outputting
    /// XML report into the given `output`.
    ///
    /// Use it only if you know what you're doing. Otherwise, consider using
    /// [`JUnit::new()`] which creates an already [`Normalized`] version of
    /// [`JUnit`] [`Writer`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://llg.cubic.org/docs/junit
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub fn raw(output: Out, verbosity: impl Into<Verbosity>) -> Self {
        let verbosity = verbosity.into();
        Self {
            output,
            report: Report::new(),
            suit: None,
            scenario_started_at: None,
            events: vec![],
            event_handler: EventHandler::<W, Out>::new(JUnitTestCaseBuilder::new(verbosity)),
            verbosity,
        }
    }

    /// Applies the given [`Cli`] options to this [`JUnit`] [`Writer`].
    pub fn apply_cli(&mut self, cli: Cli) {
        if let Some(verbosity) = cli.to_verbosity() {
            self.verbosity = verbosity;
            // Update the event handler with new verbosity
            self.event_handler = EventHandler::<W, Out>::new(JUnitTestCaseBuilder::new(self.verbosity));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::SystemTime};

    use gherkin::{Feature, LineCol};
    use junit_report::Report;

    use crate::{
        Event,
        event::{self, Cucumber, Feature as FeatureEvent},
        parser,
        writer::Verbosity,
    };

    use super::*;

    #[derive(Debug)]
    struct TestWorld;

    impl World for TestWorld {
        type Error = String;

        async fn new() -> Result<Self, Self::Error> {
            Ok(TestWorld)
        }
    }

    fn create_test_feature() -> Feature {
        Feature {
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            position: LineCol { line: 1, col: 1 },
            path: Some(PathBuf::from("/test/features/example.feature")),
        }
    }

    #[tokio::test]
    async fn creates_junit_writer_with_default_verbosity() {
        let output = Vec::new();
        let writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);

        assert_eq!(writer.verbosity, Verbosity::Default);
        assert!(writer.events.is_empty());
        assert!(writer.suit.is_none());
        assert!(writer.scenario_started_at.is_none());
    }

    #[tokio::test]
    async fn applies_cli_verbosity_settings() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let cli = Cli::with_verbosity(Some(1));

        writer.apply_cli(cli);

        assert_eq!(writer.verbosity, Verbosity::ShowWorld);
    }

    #[tokio::test]
    async fn handles_feature_started_event() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let feature = create_test_feature();
        let event = Ok(Event {
            value: Cucumber::Feature(feature.clone(), FeatureEvent::Started),
            at: SystemTime::UNIX_EPOCH,
        });
        let cli = Cli::default();

        writer.handle_event(event, &cli).await;

        assert!(writer.suit.is_some());
        let suite = writer.suit.as_ref().unwrap();
        assert_eq!(suite.name(), "Feature: Test Feature: example.feature");
    }

    #[tokio::test]
    async fn handles_feature_finished_event() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let feature = create_test_feature();
        let cli = Cli::default();

        // Start feature first
        let start_event = Ok(Event {
            value: Cucumber::Feature(feature.clone(), FeatureEvent::Started),
            at: SystemTime::UNIX_EPOCH,
        });
        writer.handle_event(start_event, &cli).await;

        // Finish feature
        let finish_event = Ok(Event {
            value: Cucumber::Feature(feature.clone(), FeatureEvent::Finished),
            at: SystemTime::UNIX_EPOCH,
        });
        writer.handle_event(finish_event, &cli).await;

        assert!(writer.suit.is_none());
        assert_eq!(writer.report.testsuites().len(), 1);
    }

    #[tokio::test]
    async fn handles_cucumber_finished_event() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let event = Ok(Event {
            value: Cucumber::Finished,
            at: SystemTime::UNIX_EPOCH,
        });
        let cli = Cli::default();

        writer.handle_event(event, &cli).await;

        // Output should contain XML
        assert!(!writer.output.is_empty());
    }

    #[tokio::test]
    async fn handles_parser_error() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::Default);
        let parse_error = gherkin::ParseFileError::Reading {
            path: PathBuf::from("/test/broken.feature"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        };
        let error = Err(parser::Error::Parsing(Box::new(parse_error)));
        let cli = Cli::default();

        writer.handle_event(error, &cli).await;

        assert_eq!(writer.report.testsuites().len(), 1);
        assert_eq!(writer.report.testsuites()[0].name(), "Errors");
    }

    #[test]
    fn clones_writer_preserving_state() {
        let output = Vec::new();
        let original = JUnit::<TestWorld, _>::raw(output, Verbosity::ShowWorld);
        let cloned = original.clone();

        assert_eq!(original.verbosity, cloned.verbosity);
        assert_eq!(original.events.len(), cloned.events.len());
        assert_eq!(original.scenario_started_at, cloned.scenario_started_at);
    }

    #[test]
    fn creates_normalized_writer() {
        let output = Vec::new();
        let _writer = JUnit::<TestWorld, _>::new(output, Verbosity::Default);
        // This test just ensures the method compiles and creates a normalized writer
    }

    #[test]
    fn creates_tee_writer() {
        let output = Vec::new();
        let _writer = JUnit::<TestWorld, _>::for_tee(output, Verbosity::Default);
        // This test just ensures the method compiles and creates a tee-compatible writer
    }

    #[test]
    fn cli_none_does_not_change_verbosity() {
        let output = Vec::new();
        let mut writer = JUnit::<TestWorld, _>::raw(output, Verbosity::ShowWorld);
        let cli = Cli::default(); // verbose: None

        writer.apply_cli(cli);

        assert_eq!(writer.verbosity, Verbosity::ShowWorld);
    }
}