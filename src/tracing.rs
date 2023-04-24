//! [`tracing`] integration layer.

use std::{collections::HashMap, fmt, io, iter, sync::Arc};

use futures::channel::{mpsc, oneshot};
use itertools::Either;
use tracing::{
    field::{Field, Visit},
    span, Dispatch, Event, Span, Subscriber,
};
use tracing_subscriber::{
    field::RecordFields,
    filter::LevelFilter,
    fmt::{
        format::{self, Format},
        FmtContext, FormatEvent, FormatFields, MakeWriter,
    },
    layer::{self, Layer, Layered, SubscriberExt as _},
    registry::LookupSpan,
    util::SubscriberInitExt as _,
};

use crate::{
    event::{self, HookType},
    runner::{
        self,
        basic::{RetryOptions, ScenarioId},
    },
    Cucumber, Parser, Runner, ScenarioType, World, Writer,
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

        drop(
            self.runner
                .logs_collector
                .swap(Box::new(Some(Collector::new(
                    logs_receiver,
                    span_close_receiver,
                )))),
        );

        self
    }
}

/// [`HashMap`] from a [`ScenarioId`] to its [`Scenario`] and full path.
///
/// [`Scenario`]: gherkin::Scenario
type Scenarios = HashMap<
    ScenarioId,
    (
        Arc<gherkin::Feature>,
        Option<Arc<gherkin::Rule>>,
        Arc<gherkin::Scenario>,
        Option<RetryOptions>,
    ),
>;

/// All [`Callback`]s for [`Span`]s closing events with their completion status.
type SpanEventsCallbacks =
    HashMap<span::Id, (Option<Vec<Callback>>, IsReceived)>;

/// Indication whether a [`Span`] closing event was received.
type IsReceived = bool;

/// Callback for notifying a [`Runner`] about a [`Span`] being closed.
type Callback = oneshot::Sender<()>;

/// Collector of [`tracing::Event`]s.
#[derive(Debug)]
pub(crate) struct Collector {
    /// [`Scenarios`] with their IDs.
    scenarios: Scenarios,

    /// Receiver of [`tracing::Event`]s messages with optional corresponding
    /// [`ScenarioId`].
    logs_receiver: mpsc::UnboundedReceiver<(Option<ScenarioId>, String)>,

    /// All [`Callback`]s for [`Span`]s closing events with their completion
    /// status.
    span_events: SpanEventsCallbacks,

    /// Receiver of a [`Span`] closing event.
    span_close_receiver: mpsc::UnboundedReceiver<span::Id>,

    /// Sender for subscribing to a [`Span`] closing event.
    wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,

    /// Receiver for subscribing to a [`Span`] closing event.
    wait_span_event_receiver: mpsc::UnboundedReceiver<(span::Id, Callback)>,
}

impl Collector {
    /// Creates a new [`tracing::Event`]s [`Collector`].
    pub(crate) fn new(
        logs_receiver: mpsc::UnboundedReceiver<(Option<ScenarioId>, String)>,
        span_close_receiver: mpsc::UnboundedReceiver<span::Id>,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            scenarios: HashMap::new(),
            logs_receiver,
            span_events: HashMap::new(),
            span_close_receiver,
            wait_span_event_sender: sender,
            wait_span_event_receiver: receiver,
        }
    }

    /// Creates a new [`SpanCloseWaiter`].
    pub(crate) fn scenario_span_event_waiter(&self) -> SpanCloseWaiter {
        SpanCloseWaiter {
            wait_span_event_sender: self.wait_span_event_sender.clone(),
        }
    }

    /// Starts [`Scenario`]s from the provided `runnable`.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn start_scenarios(
        &mut self,
        runnable: impl AsRef<
            [(
                ScenarioId,
                Arc<gherkin::Feature>,
                Option<Arc<gherkin::Rule>>,
                Arc<gherkin::Scenario>,
                ScenarioType,
                Option<RetryOptions>,
            )],
        >,
    ) {
        for (id, f, r, s, _, ret) in runnable.as_ref() {
            drop(self.scenarios.insert(
                *id,
                (
                    Arc::clone(f),
                    r.as_ref().map(Arc::clone),
                    Arc::clone(s),
                    *ret,
                ),
            ));
        }
    }

    /// Marks a [`Scenario`] as finished, by its ID.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn finish_scenario(&mut self, id: ScenarioId) {
        drop(self.scenarios.remove(&id));
    }

    /// Returns all the emitted [`event::Scenario::Log`]s since this method was
    /// last called.
    ///
    /// In case a received [`tracing::Event`] doesn't contain a [`Scenario`]'s
    /// [`Span`], such [`tracing::Event`] will be forwarded to all active
    /// [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn emitted_logs<W>(
        &mut self,
    ) -> Option<Vec<event::Cucumber<W>>> {
        self.notify_about_closing_spans();

        self.logs_receiver
            .try_next()
            .ok()
            .flatten()
            .map(|(id, msg)| {
                id.and_then(|k| self.scenarios.get(&k))
                    .map_or_else(
                        || Either::Left(self.scenarios.values()),
                        |p| Either::Right(iter::once(p)),
                    )
                    .map(|(f, r, s, opt)| {
                        event::Cucumber::scenario(
                            Arc::clone(f),
                            r.as_ref().map(Arc::clone),
                            Arc::clone(s),
                            event::RetryableScenario {
                                event: event::Scenario::Log(msg.clone()),
                                retries: opt.map(|o| o.retries),
                            },
                        )
                    })
                    .collect()
            })
    }

    /// Notifies all its subscribers about closing [`Span`]s via [`Callback`]s.
    fn notify_about_closing_spans(&mut self) {
        if let Some(id) = self.span_close_receiver.try_next().ok().flatten() {
            self.span_events.entry(id).or_default().1 = true;
        }
        while let Some((id, callback)) =
            self.wait_span_event_receiver.try_next().ok().flatten()
        {
            self.span_events
                .entry(id)
                .or_default()
                .0
                .get_or_insert(Vec::new())
                .push(callback);
        }
        self.span_events.retain(|_, (callbacks, is_received)| {
            if callbacks.is_some() && *is_received {
                for callback in callbacks
                    .take()
                    .unwrap_or_else(|| unreachable!("`callbacks.is_some()`"))
                {
                    _ = callback.send(()).ok();
                }
                false
            } else {
                true
            }
        });
    }
}

// We better keep this here, as it's related to `tracing` capabilities only.
#[allow(clippy::multiple_inherent_impl)]
impl ScenarioId {
    /// Name of the [`ScenarioId`] [`Span`] field.
    const SPAN_FIELD_NAME: &'static str = "__cucumber_scenario_id";

    /// Creates a new [`Span`] for running a [`Scenario`] with this
    /// [`ScenarioId`].
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn scenario_span(self) -> Span {
        // `Level::ERROR` is used to minimize the chance of the user-provided
        // filter to skip it.
        tracing::error_span!("scenario", __cucumber_scenario_id = self.0)
    }

    /// Creates a new [`Span`] for a running [`Step`].
    ///
    /// [`Step`]: gherkin::Step
    #[allow(clippy::unused_self)]
    pub(crate) fn step_span(self, is_background: bool) -> Span {
        // `Level::ERROR` is used to minimize the chance of the user-provided
        // filter to skip it.
        if is_background {
            tracing::error_span!("background step")
        } else {
            tracing::error_span!("step")
        }
    }

    /// Creates a new [`Span`] for running a [`Hook`].
    ///
    /// [`Hook`]: event::Hook
    #[allow(clippy::unused_self)]
    pub(crate) fn hook_span(self, hook_ty: HookType) -> Span {
        // `Level::ERROR` is used to minimize the chance of the user-provided
        // filter to skip it.
        match hook_ty {
            HookType::Before => tracing::error_span!("before hook"),
            HookType::After => tracing::error_span!("after hook"),
        }
    }
}

/// Waiter for a particular [`Span`] to be closed, wich is required because a
/// [`CollectorWriter`] can notify about an [`event::Scenario::Log`] after a
/// [`Scenario`]/[`Step`] is considered [`Finished`] already, due to
/// implementation details of a [`Subscriber`].
///
/// [`Finished`]: event::Scenario::Finished
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
#[derive(Clone, Debug)]
pub(crate) struct SpanCloseWaiter {
    /// Sender for subscribing to the [`Span`] closing.
    wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,
}

impl SpanCloseWaiter {
    /// Waits for the [`Span`] being closed.
    pub(crate) async fn wait_for_span_close(&self, id: span::Id) {
        let (sender, receiver) = oneshot::channel();
        _ = self
            .wait_span_event_sender
            .unbounded_send((id, sender))
            .ok();
        _ = receiver.await.ok();
    }
}

/// [`Layer`] recording a [`ScenarioId`] into [`Span`]'s [`Extensions`].
///
/// [`Extensions`]: tracing_subscriber::registry::Extensions
#[derive(Debug)]
pub struct RecordScenarioId {
    /// Sender for [`Span`] closing events.
    span_close_sender: mpsc::UnboundedSender<span::Id>,
}

impl RecordScenarioId {
    /// Creates a new [`RecordScenarioId`] [`Layer`].
    const fn new(span_close_sender: mpsc::UnboundedSender<span::Id>) -> Self {
        Self { span_close_sender }
    }
}

impl<S> Layer<S> for RecordScenarioId
where
    S: for<'a> LookupSpan<'a> + Subscriber,
{
    fn on_new_span(
        &self,
        attr: &span::Attributes<'_>,
        id: &span::Id,
        ctx: layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = GetScenarioId(None);
            attr.values().record(&mut visitor);

            if let Some(scenario_id) = visitor.0 {
                let mut ext = span.extensions_mut();
                _ = ext.replace(scenario_id);
            }
        }
    }

    fn on_record(
        &self,
        id: &span::Id,
        values: &span::Record<'_>,
        ctx: layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = GetScenarioId(None);
            values.record(&mut visitor);

            if let Some(scenario_id) = visitor.0 {
                let mut ext = span.extensions_mut();
                _ = ext.replace(scenario_id);
            }
        }
    }

    fn on_close(&self, id: span::Id, _ctx: layer::Context<'_, S>) {
        _ = self.span_close_sender.unbounded_send(id).ok();
    }
}

/// [`Visit`]or extracting a [`ScenarioId`] from a
/// [`ScenarioId::SPAN_FIELD_NAME`]d [`Field`], in case it's present.
#[derive(Debug)]
struct GetScenarioId(Option<ScenarioId>);

impl Visit for GetScenarioId {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == ScenarioId::SPAN_FIELD_NAME {
            self.0 = Some(ScenarioId(value));
        }
    }

    fn record_debug(&mut self, _: &Field, _: &dyn fmt::Debug) {}
}

/// [`FormatFields`] wrapper skipping [`Span`]s with a [`ScenarioId`].
#[derive(Debug)]
pub struct SkipScenarioIdSpan<F>(pub F);

impl<'w, F: FormatFields<'w>> FormatFields<'w> for SkipScenarioIdSpan<F> {
    fn format_fields<R: RecordFields>(
        &self,
        writer: format::Writer<'w>,
        fields: R,
    ) -> fmt::Result {
        let mut is_scenario_span = IsScenarioIdSpan(false);
        fields.record(&mut is_scenario_span);
        if !is_scenario_span.0 {
            self.0.format_fields(writer, fields)?;
        }
        Ok(())
    }
}

/// [`Visit`]or checking whether a [`Span`] has a [`Field`] with the
/// [`ScenarioId::SPAN_FIELD_NAME`].
#[derive(Debug)]
struct IsScenarioIdSpan(bool);

impl Visit for IsScenarioIdSpan {
    fn record_debug(&mut self, field: &Field, _: &dyn fmt::Debug) {
        if field.name() == ScenarioId::SPAN_FIELD_NAME {
            self.0 = true;
        }
    }
}

/// [`FormatEvent`] wrapper, appending [`tracing::Event`]s with some markers,
/// to parse them later and retrieve optional [`ScenarioId`].
///
/// [`Scenario`]: gherkin::Scenario
#[derive(Debug)]
pub struct AppendScenarioMsg<F>(pub F);

impl<S, N, F> FormatEvent<S, N> for AppendScenarioMsg<F>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        self.0.format_event(ctx, writer.by_ref(), event)?;

        if let Some(scenario_id) = ctx.event_scope().and_then(|scope| {
            scope
                .from_root()
                .find_map(|span| span.extensions().get::<ScenarioId>().copied())
        }) {
            writer.write_fmt(format_args!(
                "{}{scenario_id}",
                suffix::BEFORE_SCENARIO_ID,
            ))?;
        } else {
            writer.write_fmt(format_args!("{}", suffix::NO_SCENARIO_ID))?;
        }
        writer.write_fmt(format_args!("{}", suffix::END))
    }
}

mod suffix {
    //! [`str`]ings appending [`tracing::Event`]s to separate them later.
    //!
    //! Every [`tracing::Event`] ends with:
    //!
    //! ([`BEFORE_SCENARIO_ID`][`ScenarioId`][`END`]|[`NO_SCENARIO_ID`][`END`])
    //!
    //! [`ScenarioId`]: super::ScenarioId

    /// End of a [`tracing::Event`] message.
    pub(crate) const END: &str = "__cucumber__scenario";

    /// Separator before a [`ScenarioId`].
    ///
    /// [`ScenarioId`]: super::ScenarioId
    pub(crate) const BEFORE_SCENARIO_ID: &str = "__";

    /// Separator in case there is no [`ScenarioId`].
    ///
    /// [`ScenarioId`]: super::ScenarioId
    pub(crate) const NO_SCENARIO_ID: &str = "__unknown";
}

/// [`io::Write`]r sending [`tracing::Event`]s to a `Collector`.
#[derive(Clone, Debug)]
pub struct CollectorWriter {
    /// Sender for notifying the [`Collector`] about [`tracing::Event`]s via.
    sender: mpsc::UnboundedSender<(Option<ScenarioId>, String)>,
}

impl CollectorWriter {
    /// Creates a new [`CollectorWriter`].
    const fn new(
        sender: mpsc::UnboundedSender<(Option<ScenarioId>, String)>,
    ) -> Self {
        Self { sender }
    }
}

impl<'a> MakeWriter<'a> for CollectorWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl io::Write for CollectorWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Although this is not documented explicitly anywhere, `io::Write`rs
        // inside `tracing::fmt::Layer` always receives fully formatted messages
        // at once, not by parts.
        // Inside docs of `fmt::Layer::with_writer()`, a non-locked `io::stderr`
        // is passed as an `io::Writer`. So, if this guarantee fails, parts of
        // log messages will be able to interleave each other, making the result
        // unreadable.
        let msgs = String::from_utf8_lossy(buf);
        for msg in msgs.split_terminator(suffix::END) {
            if let Some((before, after)) =
                msg.rsplit_once(suffix::NO_SCENARIO_ID)
            {
                if !after.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "wrong separator",
                    ));
                }
                _ = self.sender.unbounded_send((None, before.to_owned())).ok();
            } else if let Some((before, after)) =
                msg.rsplit_once(suffix::BEFORE_SCENARIO_ID)
            {
                let scenario_id = after.parse().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, e)
                })?;
                _ = self
                    .sender
                    .unbounded_send((Some(scenario_id), before.to_owned()))
                    .ok();
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "missing separator",
                ));
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
