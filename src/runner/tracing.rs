use std::{
    collections::HashMap, fmt, future::Future, io, mem, pin::Pin, sync::Arc,
    task,
};

use futures::{channel::mpsc, Stream, StreamExt as _};
use pin_project::pin_project;
use tracing::{info_span, Span};
use tracing_core::{
    dispatcher, field::Visit, span, Event as TracingEvent, Field, Metadata,
    Subscriber,
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

pub(crate) struct Collector {
    scenarios: HashMap<
        ScenarioId,
        (
            Arc<gherkin::Feature>,
            Option<Arc<gherkin::Rule>>,
            Arc<gherkin::Scenario>,
        ),
    >,
    logs_receiver: mpsc::UnboundedReceiver<(ScenarioId, String)>,
}

impl Collector {
    pub(crate) fn new(
        logs_receiver: mpsc::UnboundedReceiver<(ScenarioId, String)>,
    ) -> Self {
        Self {
            scenarios: HashMap::new(),
            logs_receiver,
        }
    }

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
        for (id, f, r, s, ..) in runnable.as_ref() {
            drop(self.scenarios.insert(
                *id,
                (Arc::clone(f), r.as_ref().map(Arc::clone), Arc::clone(s)),
            ));
        }
    }

    pub(crate) fn finish_scenario(&mut self, id: ScenarioId) {
        drop(self.scenarios.remove(&id));
    }

    pub(crate) async fn collect_logs<W>(&mut self) -> Vec<event::Cucumber<W>> {
        self.logs_receiver
            .by_ref()
            .map(|(id, msg)| {
                let (f, r, s) = self
                    .scenarios
                    .get(&id)
                    .unwrap_or_else(|| panic!("No `Scenario` with ID: {id}"));
                event::Cucumber::scenario(
                    Arc::clone(f),
                    r.as_ref().map(Arc::clone),
                    Arc::clone(s),
                    event::RetryableScenario {
                        event: event::Scenario::Log(msg),
                        retries: None,
                    },
                )
            })
            .collect()
            .await
    }
}

#[pin_project]
pub(crate) struct DefaultDispatch<F> {
    #[pin]
    inner: F,
    dispatch: Dispatch,
}

impl<S> DefaultDispatch<S> {
    pub(crate) fn new(inner: S, dispatch: Dispatch) -> Self {
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
        let _guard = dispatcher::set_default(&this.dispatch);
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
        let _guard = dispatcher::set_default(&this.dispatch);
        this.inner.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ScenarioId {
    const START_MSG_ESCAPE: &'static str = "__cucumber_scenario_start__";
    const END_MSG_ESCAPE: &'static str = "__cucumber_scenario_end__";
    const AFTER_SCENARIO_ID_ESCAPE: &'static str = "_";
    const FIELD_NAME: &'static str = "__cucumber_scenario_id";

    pub(crate) fn span(self) -> Span {
        info_span!("scenario", __cucumber_scenario_id = self.0)
    }
}

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

            // let scenario_id = visitor.0.or_else(|| {
            //     ctx.span_scope(id).into_iter().flatten().find_map(|scope| {
            //         scope.extensions().get::<RequestId>().copied()
            //     })
            // });

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

/// [`Visit`]or extracting [`ScenarioId`] from a [`ScenarioId::FIELD_NAME`]
/// [`Field`] in case it's present.
#[derive(Debug)]
struct GetScenarioId(Option<ScenarioId>);

impl Visit for GetScenarioId {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == ScenarioId::FIELD_NAME {
            self.0 = Some(ScenarioId(value));
        }
    }

    fn record_debug(&mut self, _: &Field, _: &dyn fmt::Debug) {}
}

#[derive(Debug)]
pub(crate) struct SkipScenarioIdSpan<F>(pub(crate) F);

impl<'w, F: FormatFields<'w>> FormatFields<'w> for SkipScenarioIdSpan<F> {
    fn format_fields<R: RecordFields>(
        &self,
        writer: format::Writer<'w>,
        fields: R,
    ) -> fmt::Result {
        let mut is_meta = IsScenarioIdSpan(false);
        fields.record(&mut is_meta);
        if !is_meta.0 {
            self.0.format_fields(writer, fields)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct IsScenarioIdSpan(bool);

impl Visit for IsScenarioIdSpan {
    fn record_debug(&mut self, field: &Field, _: &dyn fmt::Debug) {
        if field.name() == ScenarioId::FIELD_NAME {
            self.0 = true;
        }
    }
}

pub(crate) struct AppendScenarioId<F>(pub(crate) F);

impl<S, N, F> FormatEvent<S, N> for AppendScenarioId<F>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    F: FormatEvent<S, N>,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &TracingEvent<'_>,
    ) -> fmt::Result {
        let scenario_id = ctx.event_scope().and_then(|scope| {
            scope
                .from_root()
                .find_map(|span| span.extensions().get::<ScenarioId>().copied())
        });
        if scenario_id.is_some() {
            writer.write_str(ScenarioId::START_MSG_ESCAPE)?;
        }
        self.0.format_event(ctx, writer.by_ref(), event)?;
        if let Some(scenario_id) = scenario_id {
            writer.write_fmt(format_args!(
                "{}{}{}",
                ScenarioId::END_MSG_ESCAPE,
                scenario_id,
                ScenarioId::AFTER_SCENARIO_ID_ESCAPE,
            ))?;
        }
        Ok(())
    }
}

pub(crate) struct MakePostponedWriter<W> {
    sender: mpsc::UnboundedSender<(ScenarioId, String)>,
    other: W,
}

impl<W> MakePostponedWriter<W> {
    pub(crate) fn new(
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

enum PostponedWriterState {
    WaitingForStart,
    CollectingMsg(String),
    CollectingScenarioId(String, String),
    FoundEscape(String, ScenarioId),
}

pub(crate) struct PostponedWriter<W> {
    sender: mpsc::UnboundedSender<(ScenarioId, String)>,
    other: W,
    state: PostponedWriterState,
}

impl<W: io::Write> io::Write for PostponedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use PostponedWriterState as State;

        let msg = String::from_utf8_lossy(buf);
        let mut msg = msg.as_ref();
        loop {
            match &mut self.state {
                State::WaitingForStart => {
                    if let Some((before, after)) =
                        msg.split_once(ScenarioId::START_MSG_ESCAPE)
                    {
                        let _ = self.other.write(before.as_bytes())?;
                        msg = after;
                        self.state =
                            State::CollectingMsg(String::with_capacity(128));
                    } else {
                        let _ = self.other.write(msg.as_bytes())?;
                        break;
                    }
                }
                State::CollectingMsg(buf) => {
                    if let Some((before, after)) =
                        msg.split_once(ScenarioId::END_MSG_ESCAPE)
                    {
                        buf.push_str(before);
                        msg = after;
                        self.state = State::CollectingScenarioId(
                            mem::take(buf),
                            String::new(),
                        );
                    } else {
                        buf.push_str(msg);
                        break;
                    }
                }
                State::CollectingScenarioId(msg_buf, id_buf) => {
                    if let Some((before, after)) =
                        msg.split_once(ScenarioId::AFTER_SCENARIO_ID_ESCAPE)
                    {
                        id_buf.push_str(before);
                        msg = after;
                        self.state = State::FoundEscape(
                            mem::take(msg_buf),
                            id_buf.parse().expect("valid `ScenarioId`"),
                        );
                    } else {
                        id_buf.push_str(msg);
                        break;
                    }
                }
                State::FoundEscape(msg_buf, id) => {
                    let _ = self
                        .sender
                        .unbounded_send((*id, mem::take(msg_buf)))
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
