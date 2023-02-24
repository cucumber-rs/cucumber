//! [`tracing`] integration layer.

use std::{collections::HashMap, fmt, io, iter, sync::Arc};

use futures::channel::mpsc;
use itertools::Either;
use tracing::{error_span, Span};
use tracing_core::{field::Visit, span, Event, Field, Subscriber};
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

pub use tracing::{dispatcher::set_global_default, Instrument};
pub use tracing_core::Dispatch;

/// [`tracing`] [`Event`]s collector.
pub(crate) struct Collector {
    /// [`Scenarios`] with their IDs.
    scenarios: Scenarios,

    /// Receiver for [`tracing`] [`Event`]s messages with optional corresponding
    /// [`ScenarioId`].
    logs_receiver: mpsc::UnboundedReceiver<(Option<ScenarioId>, String)>,
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
    ///
    /// In case [`tracing`] [`Event`] received doesn't contain [`Scenario`]
    /// [`Span`], this [`Event`] will be forwarded to all active [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn next_logs<W>(&mut self) -> Option<Vec<event::Cucumber<W>>> {
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

/// [`FormatFields`] wrapper that skips [`Span`]s with [`ScenarioId`].
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

/// [`FormatEvent`] wrapper that appends [`tracing`] [`Event`]s to later parse
/// them and retrieve optional [`ScenarioId`].
///
/// [`Scenario`]: gherkin::Scenario
pub(crate) struct AppendScenarioMsg<F>(pub(crate) F);

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
    pub(super) const END: &str = "__cucumber__scenario";

    /// Separator before [`ScenarioId`].
    ///
    /// [`ScenarioId`]: super::ScenarioId
    pub(super) const BEFORE_SCENARIO_ID: &str = "__";

    /// Separator in case there is no [`ScenarioId`].
    ///
    /// [`ScenarioId`]: super::ScenarioId
    pub(super) const NO_SCENARIO_ID: &str = "__unknown";
}

/// [`io::Write`]r that sends [`tracing`] [`Event`]s to the [`Collector`].
#[derive(Clone, Debug)]
pub(crate) struct CollectorWriter {
    /// Sender for notifying [`Collector`] about [`tracing`] [`Event`]s.
    sender: mpsc::UnboundedSender<(Option<ScenarioId>, String)>,
}

impl CollectorWriter {
    /// Creates a new [`CollectorWriter`].
    pub(crate) const fn new(
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
