//! [`tracing`] integration layer.

use std::{
    collections::HashMap,
    fmt, io, iter,
    sync::{Arc, Mutex},
};

use futures::channel::{mpsc, oneshot};
use itertools::Either;
use tracing::{error_span, Span};
use tracing_core::{
    field::Visit, span, Dispatch, Event, Field, LevelFilter, Subscriber,
};
use tracing_subscriber::{
    field::RecordFields,
    fmt::{
        format::{self, Format},
        FmtContext, FormatEvent, FormatFields, MakeWriter,
    },
    layer::{self, Layered, SubscriberExt as _},
    registry::LookupSpan,
    util::SubscriberInitExt as _,
    Layer,
};

use crate::{
    event,
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
    /// Initializes global [`Subscriber`].
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

    /// Configures [`fmt::Layer`], additionally wraps it (for example in
    /// [`LevelFilter`]) and initializes as global [`Subscriber`].
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

/// [`tracing`] [`Event`]s collector.
#[derive(Debug)]
pub(crate) struct Collector {
    /// [`Scenarios`] with their IDs.
    scenarios: Scenarios,

    /// Receiver for [`tracing`] [`Event`]s messages with optional corresponding
    /// [`ScenarioId`].
    logs_receiver: mpsc::UnboundedReceiver<(Option<ScenarioId>, String)>,

    span_events:
        HashMap<(ScenarioId, SpanEvent), (Option<Vec<Callback>>, IsReceived)>,

    span_event_receiver: mpsc::UnboundedReceiver<(ScenarioId, SpanEvent)>,

    wait_span_event_sender:
        mpsc::UnboundedSender<(ScenarioId, SpanEvent, Callback)>,

    wait_span_event_receiver:
        mpsc::UnboundedReceiver<(ScenarioId, SpanEvent, Callback)>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum SpanEvent {
    Exit,
    Close,
}

type IsReceived = bool;

type Callback = oneshot::Sender<()>;

#[derive(Clone, Debug)]
pub(crate) struct SpanEventWaiter {
    wait_span_event_sender:
        mpsc::UnboundedSender<(ScenarioId, SpanEvent, Callback)>,
}

impl SpanEventWaiter {
    pub(crate) async fn wait_for_scenario_span_exit(&self, id: ScenarioId) {
        let (sender, receiver) = oneshot::channel();
        let _ = self
            .wait_span_event_sender
            .unbounded_send((id, SpanEvent::Exit, sender))
            .ok();
        let _ = receiver.await.ok();
    }

    pub(crate) async fn wait_for_scenario_span_close(&self, id: ScenarioId) {
        let (sender, receiver) = oneshot::channel();
        let _ = self
            .wait_span_event_sender
            .unbounded_send((id, SpanEvent::Close, sender))
            .ok();
        let _ = receiver.await.ok();
    }
}

/// [`HashMap`] from [`ScenarioId`] to [`Scenario`] and it's full path.
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

impl Collector {
    /// Creates a new [`tracing`] [`Event`]s [`Collector`].
    pub(crate) fn new(
        logs_receiver: mpsc::UnboundedReceiver<(Option<ScenarioId>, String)>,
        span_event_receiver: mpsc::UnboundedReceiver<(ScenarioId, SpanEvent)>,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            scenarios: HashMap::new(),
            logs_receiver,
            span_events: HashMap::new(),
            span_event_receiver,
            wait_span_event_sender: sender,
            wait_span_event_receiver: receiver,
        }
    }

    pub(crate) fn scenario_span_event_waiter(&self) -> SpanEventWaiter {
        SpanEventWaiter {
            wait_span_event_sender: self.wait_span_event_sender.clone(),
        }
    }

    /// Starts [`Scenario`]s.
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

    /// Marks [`Scenario`] as finished.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn finish_scenario(&mut self, id: ScenarioId) {
        drop(self.scenarios.remove(&id));
    }

    /// Returns all emitted [`event::Scenario::Log`] since this method was last
    /// called.
    ///
    /// In case [`tracing`] [`Event`] received doesn't contain [`Scenario`]
    /// [`Span`], this [`Event`] will be forwarded to all active [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn emitted_logs<W>(
        &mut self,
    ) -> Option<Vec<event::Cucumber<W>>> {
        self.notify_about_span_events();

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

    fn notify_about_span_events(&mut self) {
        if let Some((id, event)) =
            self.span_event_receiver.try_next().ok().flatten()
        {
            self.span_events.entry((id, event)).or_default().1 = true;
        }
        while let Some((id, event, callback)) =
            self.wait_span_event_receiver.try_next().ok().flatten()
        {
            self.span_events
                .entry((id, event))
                .or_default()
                .0
                .get_or_insert(Vec::new())
                .push(callback);
        }
        self.span_events.retain(|_, (callbacks, is_received)| {
            if callbacks.is_some() && *is_received {
                for callback in callbacks.take().unwrap() {
                    let _ = callback.send(()).ok();
                }
                false
            } else {
                true
            }
        })
    }
}

#[allow(clippy::multiple_inherent_impl)]
impl ScenarioId {
    /// Name of the [`ScenarioId`] [`Span`] field.
    const SPAN_FIELD_NAME: &'static str = "__cucumber_scenario_id";

    /// Creates a [`Span`] for running [`Scenario`] with ID.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn span(self) -> Span {
        // `Level::ERROR` is used to minimize chance of user-provided filter to
        // skip it.
        error_span!("scenario", __cucumber_scenario_id = self.0)
    }
}

/// [`Layer`] recording [`ScenarioId`] in [`Span`]'s [`Extensions`].
///
/// [`Extensions`]: tracing_subscriber::registry::Extensions
#[derive(Debug)]
pub struct RecordScenarioId {
    span_to_scenario_ids: Mutex<HashMap<span::Id, ScenarioId>>,
    span_close_sender: mpsc::UnboundedSender<(ScenarioId, SpanEvent)>,
}

impl RecordScenarioId {
    fn new(
        span_close_sender: mpsc::UnboundedSender<(ScenarioId, SpanEvent)>,
    ) -> Self {
        Self {
            span_to_scenario_ids: Mutex::new(HashMap::new()),
            span_close_sender,
        }
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
                let _ = ext.replace(scenario_id);
                drop(
                    self.span_to_scenario_ids
                        .lock()
                        .unwrap()
                        .insert(id.clone(), scenario_id),
                );
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
                let _ = ext.replace(scenario_id);
            }
        }
    }

    fn on_exit(&self, id: &span::Id, _ctx: layer::Context<'_, S>) {
        let id = self
            .span_to_scenario_ids
            .lock()
            .unwrap()
            .get(id)
            .copied()
            .unwrap();
        let _ = self
            .span_close_sender
            .unbounded_send((id, SpanEvent::Exit))
            .ok();
    }

    fn on_close(&self, id: span::Id, _ctx: layer::Context<'_, S>) {
        let id = self
            .span_to_scenario_ids
            .lock()
            .unwrap()
            .remove(&id)
            .unwrap();
        let _ = self
            .span_close_sender
            .unbounded_send((id, SpanEvent::Close))
            .ok();
    }
}

/// [`Visit`]or extracting [`ScenarioId`] from a [`ScenarioId::SPAN_FIELD_NAME`]
/// [`Field`] in case it's present.
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

/// [`FormatFields`] wrapper that skips [`Span`]s with [`ScenarioId`].
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
        // if !is_scenario_span.0 {
        self.0.format_fields(writer, fields)?;
        // }
        Ok(())
    }
}

/// [`Visit`]or that checks whether [`Span`] has [`Field`] with name
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

/// [`FormatEvent`] wrapper that appends [`tracing`] [`Event`]s to later parse
/// them and retrieve optional [`ScenarioId`].
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
                "{}{}{}",
                suffix::BEFORE_SCENARIO_ID,
                scenario_id,
                suffix::END,
            ))
        } else {
            writer.write_fmt(format_args!(
                "{}{}",
                suffix::NO_SCENARIO_ID,
                suffix::END,
            ))
        }
    }
}

mod suffix {
    //! [`str`]ings appending [`tracing`] [`Event`]s to separate them later.
    //!
    //! Every [`tracing`] [`Event`] ends with:
    //!
    //! ([`BEFORE_SCENARIO_ID`][`ScenarioId`][`END`]|[`NO_SCENARIO_ID`][`END`])
    //!
    //! [`Event`]: tracing::Event
    //! [`ScenarioId`]: super::ScenarioId

    /// End of [`tracing`] [`Event`] message.
    ///
    /// [`Event`]: tracing::Event
    pub(crate) const END: &str = "__cucumber__scenario";

    /// Separator before [`ScenarioId`].
    ///
    /// [`ScenarioId`]: super::ScenarioId
    pub(crate) const BEFORE_SCENARIO_ID: &str = "__";

    /// Separator in case there is no [`ScenarioId`].
    ///
    /// [`ScenarioId`]: super::ScenarioId
    pub(crate) const NO_SCENARIO_ID: &str = "__unknown";
}

/// [`io::Write`]r that sends [`tracing`] [`Event`]s to the `Collector`.
#[derive(Clone, Debug)]
pub struct CollectorWriter {
    /// Sender for notifying [`Collector`] about [`tracing`] [`Event`]s.
    sender: mpsc::UnboundedSender<(Option<ScenarioId>, String)>,
}

impl CollectorWriter {
    /// Creates a new [`CollectorWriter`].
    #[must_use]
    pub const fn new(
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
        // inside `tracing::fmt::Layer` always receives fully formatted
        // messages at once, not as parts.
        // Inside docs of `fmt::Layer::with_writer`, non-locked `io::stderr` is
        // passed as an `io::Writer`. So if this guarantee fails, parts of log
        // messages will be able to interleave each other, making result
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
                let _ =
                    self.sender.unbounded_send((None, before.to_owned())).ok();
            } else if let Some((before, after)) =
                msg.rsplit_once(suffix::BEFORE_SCENARIO_ID)
            {
                let scenario_id = after.parse().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, e)
                })?;
                let _ = self
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
