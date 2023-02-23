//! [`tracing`] integration layer.

use std::{
    collections::HashMap, fmt, future::Future, io, mem, pin::Pin, sync::Arc,
    task,
};

use futures::{channel::mpsc, Stream};
use pin_project::pin_project;
use tracing::{error_span, Span};
use tracing_core::{
    dispatcher, field::Visit, span, Event, Field, Metadata, Subscriber,
};
use tracing_subscriber::{
    field::RecordFields,
    fmt::{format, FmtContext, FormatEvent, FormatFields, MakeWriter},
    layer,
    registry::LookupSpan,
    Layer,
};

use crate::{
    event,
    runner::basic::{RetryOptions, ScenarioId},
    ScenarioType,
};

pub use tracing::Instrument;
pub use tracing_core::Dispatch;

/// [`tracing`] [`Event`]s collector.
pub(crate) struct Collector {
    /// [`Scenarios`] with their IDs.
    scenarios: Scenarios,

    /// Receiver for [`tracing`] [`Event`]s messages with corresponding
    /// [`ScenarioId`].
    logs_receiver: mpsc::UnboundedReceiver<(ScenarioId, String)>,
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
        logs_receiver: mpsc::UnboundedReceiver<(ScenarioId, String)>,
    ) -> Self {
        Self {
            scenarios: HashMap::new(),
            logs_receiver,
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

    /// Tries to receive next [`event::Scenario::Log`].
    pub(crate) fn next_log<W>(&mut self) -> Option<event::Cucumber<W>> {
        loop {
            if let Ok(res) = self
                .logs_receiver
                .try_next()
                .ok()
                .flatten()
                .map(|(id, msg)| {
                    // In case `Scenario` is already finished, but `tracing`
                    // events from that `Span` still received, we just ignore
                    // them.
                    self.scenarios.get(&id).map_or(Err(()), |(f, r, s, opt)| {
                        Ok(event::Cucumber::scenario(
                            Arc::clone(f),
                            r.as_ref().map(Arc::clone),
                            Arc::clone(s),
                            event::RetryableScenario {
                                event: event::Scenario::Log(msg),
                                retries: opt.map(|o| o.retries),
                            },
                        ))
                    })
                })
                .transpose()
            {
                return res;
            }
        }
    }
}

/// Sets default [`Dispatch`] for wrapped [`Future`] or [`Stream`].
#[pin_project]
pub(crate) struct DefaultDispatch<F> {
    /// Wrapped [`Future`] or [`Stream`].
    #[pin]
    inner: F,

    /// Default [`Dispatch`].
    dispatch: Dispatch,
}

impl<S> DefaultDispatch<S> {
    /// Creates a new [`DefaultDispatch`].
    pub(crate) const fn new(inner: S, dispatch: Dispatch) -> Self {
        Self { inner, dispatch }
    }
}

impl<F: Future> Future for DefaultDispatch<F> {
    type Output = F::Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Self::Output> {
        let this = self.project();
        let _guard = dispatcher::set_default(this.dispatch);
        this.inner.poll(cx)
    }
}

impl<S: Stream> Stream for DefaultDispatch<S> {
    type Item = S::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        let this = self.project();
        let _guard = dispatcher::set_default(this.dispatch);
        this.inner.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
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
pub(crate) struct RecordScenarioId;

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

/// [`FormatFields`] wrapper that skips [`Span`] with [`ScenarioId`].
#[derive(Debug)]
pub(crate) struct SkipScenarioIdSpan<F>(pub(crate) F);

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

/// [`FormatEvent`] wrapper that escapes [`tracing`] [`Event`]s that are emitted
/// inside [`Scenario`] [`Span`].
///
/// [`Scenario`]: gherkin::Scenario
pub(crate) struct EscapeScenarioMsg<F>(pub(crate) F);

impl<S, N, F> FormatEvent<S, N> for EscapeScenarioMsg<F>
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
        let scenario_id = ctx.event_scope().and_then(|scope| {
            scope
                .from_root()
                .find_map(|span| span.extensions().get::<ScenarioId>().copied())
        });
        if scenario_id.is_some() {
            writer.write_str(escape::START)?;
        }
        self.0.format_event(ctx, writer.by_ref(), event)?;
        if let Some(scenario_id) = scenario_id {
            writer.write_fmt(format_args!(
                "{}{}{}",
                escape::END,
                scenario_id,
                escape::AFTER_SCENARIO_ID,
            ))?;
        }
        Ok(())
    }
}

mod escape {
    //! [`str`]ings for escaping [`tracing`] [`Event`]s.
    //!
    //! Format is the following:
    //!
    //! [`START`]log[`END`][`ScenarioId`][`AFTER_SCENARIO_ID`]
    //!
    //! [`Event`]: tracing::Event
    //! [`ScenarioId`]: super::ScenarioId

    /// Start of the [`tracing`] [`Event`] message.
    ///
    /// [`Event`]: tracing::Event
    pub(super) const START: &str = "__cucumber_scenario_start__";

    /// End of the [`tracing`] [`Event`] message.
    ///
    /// [`Event`]: tracing::Event
    pub(super) const END: &str = "__cucumber_scenario_end__";

    /// End of the [`ScenarioId`].
    ///
    /// [`ScenarioId`]: super::ScenarioId
    pub(super) const AFTER_SCENARIO_ID: &str = "_";
}

/// [`MakeWriter`] implementor returning [`PostponedWriter`].
pub(crate) struct MakePostponedWriter<W> {
    /// Sender for notifying [`Collector`] about [`tracing`] [`Event`]s.
    sender: mpsc::UnboundedSender<(ScenarioId, String)>,

    /// Inner [`MakeWriter`].
    other: W,
}

impl<W> MakePostponedWriter<W> {
    /// Creates a new [`MakePostponedWriter`].
    pub(crate) const fn new(
        sender: mpsc::UnboundedSender<(ScenarioId, String)>,
        other: W,
    ) -> Self {
        Self { sender, other }
    }
}

impl<'a, W: MakeWriter<'a>> MakeWriter<'a> for MakePostponedWriter<W> {
    type Writer = PostponedWriter<W::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        PostponedWriter {
            sender: self.sender.clone(),
            other: self.other.make_writer(),
            state: PostponedWriterState::WaitingForStart,
        }
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        PostponedWriter {
            sender: self.sender.clone(),
            other: self.other.make_writer_for(meta),
            state: PostponedWriterState::WaitingForStart,
        }
    }
}

/// [`io::Write`]er for parsing [`escape`]d [`tracing`] [`Event`]s and sending
/// then to the [`Collector`].
pub(crate) struct PostponedWriter<W> {
    /// Sender for notifying [`Collector`] about [`tracing`] [`Event`]s.
    sender: mpsc::UnboundedSender<(ScenarioId, String)>,

    /// Other [`io::Write`]r for unescaped [`tracing`] [`Event`]s.
    other: W,

    /// State of this [`io::Write`]r.
    state: PostponedWriterState,
}

/// State of the [`PostponedWriter`].
enum PostponedWriterState {
    /// No [`escape::START`] found.
    ///
    /// All [`tracing`] [`Event`]s received in this state will be forwarded to
    /// inner [`io::Write`]r.
    WaitingForStart,

    /// [`escape::START`] encountered, collecting [`tracing`] [`Event`] into
    /// `buffer`.
    CollectingMsg {
        /// [`tracing`] [`Event`] buffer.
        buffer: String,
    },

    /// [`escape::END`] encountered, collecting [`ScenarioId`] into `buffer`.
    CollectingScenarioId {
        /// [`ScenarioId`] buffer.
        buffer: String,

        /// Complete [`tracing`] [`Event`].
        msg: String,
    },

    /// [`escape::AFTER_SCENARIO_ID`] found, sending `id` and `msg` to
    /// [`Collector`].
    FoundEscape {
        /// ID of the [`Scenario`].
        ///
        /// [`Scenario`]: gherkin::Scenario
        id: ScenarioId,

        /// [`tracing`] [`Event`].
        msg: String,
    },
}

impl<W: io::Write> io::Write for PostponedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use PostponedWriterState as State;

        let msg = String::from_utf8_lossy(buf);
        let mut msg = msg.as_ref();
        loop {
            match &mut self.state {
                State::WaitingForStart => {
                    if let Some((before, after)) = msg.split_once(escape::START)
                    {
                        if let Some((id, complete_msg)) = before
                            .is_empty()
                            .then(|| after.split_once(escape::END))
                            .flatten()
                            .and_then(|(msg, rest)| {
                                let (id, rest) =
                                    rest.split_once(escape::AFTER_SCENARIO_ID)?;
                                rest.is_empty()
                                    .then(|| id.parse::<ScenarioId>().ok())
                                    .flatten()
                                    .map(|id| (id, msg))
                            })
                        {
                            // Optimization for exactly one complete escaped
                            // `tracing` `Event`.
                            msg = "";
                            self.state = State::FoundEscape {
                                id,
                                msg: complete_msg.to_owned(),
                            };
                        } else {
                            self.other.write_all(before.as_bytes())?;
                            msg = after;
                            self.state = State::CollectingMsg {
                                buffer: String::with_capacity(128),
                            };
                        }
                    } else {
                        self.other.write_all(msg.as_bytes())?;
                        break;
                    }
                }
                State::CollectingMsg { buffer } => {
                    if let Some((before, after)) = msg.split_once(escape::END) {
                        buffer.push_str(before);
                        msg = after;
                        self.state = State::CollectingScenarioId {
                            msg: mem::take(buffer),
                            buffer: String::new(),
                        };
                    } else {
                        buffer.push_str(msg);
                        break;
                    }
                }
                State::CollectingScenarioId {
                    msg: complete_msg,
                    buffer: id_buffer,
                } => {
                    if let Some((before, after)) =
                        msg.split_once(escape::AFTER_SCENARIO_ID)
                    {
                        id_buffer.push_str(before);
                        msg = after;
                        self.state = State::FoundEscape {
                            msg: mem::take(complete_msg),
                            id: id_buffer.parse().map_err(|e| {
                                io::Error::new(io::ErrorKind::InvalidData, e)
                            })?,
                        };
                    } else {
                        id_buffer.push_str(msg);
                        break;
                    }
                }
                State::FoundEscape {
                    msg: complete_msg,
                    id,
                } => {
                    let _ = self
                        .sender
                        .unbounded_send((*id, mem::take(complete_msg)))
                        .ok();
                    self.state = State::WaitingForStart;
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.other.flush()
    }
}
