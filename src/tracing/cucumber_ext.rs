//! Extension methods for [`Cucumber`] to configure tracing integration.

use futures::channel::mpsc;
use tracing::{Dispatch, Subscriber};
use tracing_subscriber::{
    field::RecordFields,
    filter::LevelFilter,
    fmt::{
        FmtContext, FormatEvent, FormatFields, MakeWriter,
        format::{self, Format},
    },
    layer::{Layered, SubscriberExt as _},
    registry::LookupSpan,
    util::SubscriberInitExt as _,
};

use crate::{
    Cucumber, Parser, Runner, World, Writer,
    runner,
};

use super::{
    collector::Collector,
    layer::RecordScenarioId,
    formatter::{AppendScenarioMsg, SkipScenarioIdSpan},
    writer::CollectorWriter,
};

impl<W, P, I, Wr, Cli, WhichSc, Before, After>
    Cucumber<W, P, I, runner::Basic<W, WhichSc, Before, After>, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    runner::Basic<W, WhichSc, Before, After>: Runner<W>,
    Wr: Writer<W>,
    Cli: clap::Args,
{
    /// Initializes a global [`tracing::Subscriber`] with a default
    /// [`fmt::Layer`] and [`LevelFilter::INFO`].
    ///
    /// [`fmt::Layer`]: tracing_subscriber::fmt::Layer
    #[must_use]
    pub fn init_tracing(self) -> Self {
        self.configure_and_init_tracing(
            format::DefaultFields::new(),
            Format::default(),
            |layer| {
                tracing_subscriber::registry()
                    .with(LevelFilter::INFO.and_then(layer))
            },
        )
    }

    /// Configures a [`fmt::Layer`], additionally wraps it (for example, into a
    /// [`LevelFilter`]), and initializes as a global [`tracing::Subscriber`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # use cucumber::{Cucumber, World as _};
    /// # use tracing_subscriber::{
    /// #     filter::LevelFilter,
    /// #     fmt::format::{self, Format},
    /// #     layer::SubscriberExt,
    /// #     Layer,
    /// # };
    /// #
    /// # #[derive(Debug, Default, cucumber::World)]
    /// # struct World;
    /// #
    /// # let _ = async {
    /// World::cucumber()
    ///     .configure_and_init_tracing(
    ///         format::DefaultFields::new(),
    ///         Format::default(),
    ///         |fmt_layer| {
    ///             tracing_subscriber::registry()
    ///                 .with(LevelFilter::INFO.and_then(fmt_layer))
    ///         },
    ///     )
    ///     .run_and_exit("./tests/features/doctests.feature")
    ///     .await
    /// # };
    /// ```
    ///
    /// [`fmt::Layer`]: tracing_subscriber::fmt::Layer
    #[must_use]
    pub fn configure_and_init_tracing<Event, Fields, Sub, Conf, Out>(
        self,
        fmt_fields: Fields,
        event_format: Event,
        configure: Conf,
    ) -> Self
    where
        Fields: for<'a> FormatFields<'a> + 'static,
        Event: FormatEvent<Sub, SkipScenarioIdSpan<Fields>> + 'static,
        Sub: Subscriber + for<'a> LookupSpan<'a>,
        Out: Subscriber + Send + Sync,
        // TODO: Replace the inner type with TAIT, once stabilized:
        //       https://github.com/rust-lang/rust/issues/63063
        Conf: FnOnce(
            Layered<
                tracing_subscriber::fmt::Layer<
                    Sub,
                    SkipScenarioIdSpan<Fields>,
                    AppendScenarioMsg<Event>,
                    CollectorWriter,
                >,
                RecordScenarioId,
                Sub,
            >,
        ) -> Out,
    {
        let (logs_sender, logs_receiver) = mpsc::unbounded();
        let (span_close_sender, span_close_receiver) = mpsc::unbounded();

        let layer = RecordScenarioId::new(span_close_sender).and_then(
            tracing_subscriber::fmt::layer()
                .fmt_fields(SkipScenarioIdSpan(fmt_fields))
                .event_format(AppendScenarioMsg(event_format))
                .with_writer(CollectorWriter::new(logs_sender)),
        );
        Dispatch::new(configure(layer)).init();

        drop(self.runner.logs_collector.swap(Box::new(Some(Collector::new(
            logs_receiver,
            span_close_receiver,
        )))));

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::fmt::format;

    #[derive(Debug, Default)]
    struct TestWorld;

    impl World for TestWorld {
        type Error = ();
        
        async fn new() -> Result<Self, Self::Error> {
            Ok(Self::default())
        }
    }

    #[test]
    fn test_init_tracing_returns_self() {
        use crate::cucumber::DefaultParser;
        use crate::runner::Basic;
        use crate::writer;
        use crate::cucumber::DefaultCli;
        
        let cucumber: Cucumber<
            TestWorld,
            DefaultParser,
            _,
            Basic<TestWorld, _, _, _>,
            writer::Basic<TestWorld>,
            DefaultCli,
        > = TestWorld::cucumber();
        
        // This should compile and return Self
        let _result = cucumber.init_tracing();
    }

    #[test]
    fn test_configure_and_init_tracing_accepts_custom_format() {
        use crate::cucumber::DefaultParser;
        use crate::runner::Basic;
        use crate::writer;
        use crate::cucumber::DefaultCli;
        
        let cucumber: Cucumber<
            TestWorld,
            DefaultParser,
            _,
            Basic<TestWorld, _, _, _>,
            writer::Basic<TestWorld>,
            DefaultCli,
        > = TestWorld::cucumber();
        
        let _result = cucumber.configure_and_init_tracing(
            format::DefaultFields::new(),
            Format::default(),
            |layer| {
                tracing_subscriber::registry()
                    .with(LevelFilter::DEBUG.and_then(layer))
            },
        );
    }

    #[test]
    fn test_tracing_configuration_creates_channels() {
        let (logs_sender, _logs_receiver) = mpsc::unbounded();
        let (span_close_sender, _span_close_receiver) = mpsc::unbounded();
        
        // Test that channels can be created
        assert!(logs_sender.unbounded_send((None, "test".to_string())).is_ok());
        assert!(span_close_sender.unbounded_send(tracing::span::Id::from_u64(1)).is_ok());
    }

    #[test]
    fn test_collector_writer_creation() {
        let (logs_sender, _logs_receiver) = mpsc::unbounded();
        let writer = CollectorWriter::new(logs_sender);
        
        // Test that CollectorWriter can be created
        assert!(std::mem::size_of_val(&writer) > 0);
    }
}