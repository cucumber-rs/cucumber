//! Additive statistics for native global hooks without changing event enums.
use crate::{Event, Writer, event, parser, runner::RunHookStore, writer};
/// Keeps ordinary writer behavior while counting actual global hook failures.
///
/// `Cucumber::with_run_hooks` installs this adapter after selecting the writer.
/// Replacing the writer afterward removes the adapter, so install run hooks
/// after choosing a custom writer. Specialized protocol writers also need the
/// shared store to emit individual global-hook records; this adapter does not
/// invent native events or protocol messages.
#[derive(Clone, Debug)]
pub struct RunHooks<Wr> {
    writer: Wr,
    store: RunHookStore,
    run_id: Option<u64>,
    finished_failures: Option<usize>,
}
impl<Wr> RunHooks<Wr> {
    /// Wraps a writer with statistics from the runner's shared global-hook store.
    #[must_use]
    pub fn new(writer: Wr, store: RunHookStore) -> Self {
        Self { writer, store, run_id: None, finished_failures: None }
    }
    /// Returns the wrapped writer.
    #[must_use]
    pub fn inner(&self) -> &Wr {
        &self.writer
    }
    /// Unwraps the original writer after preserving any needed hook statistics.
    #[must_use]
    pub fn into_inner(self) -> Wr {
        self.writer
    }
    /// Counts failures belonging to this adapter's native run.
    #[must_use]
    pub fn run_hook_errors(&self) -> usize {
        self.finished_failures.unwrap_or_else(|| {
            self.run_id.map_or(0, |id| self.store.failures_for(id))
        })
    }
}
impl<W, Wr: Writer<W>> Writer<W> for RunHooks<Wr> {
    type Cli = Wr::Cli;
    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        cli: &Self::Cli,
    ) {
        if let Ok(value) = &event {
            match &value.value {
                event::Cucumber::Started => {
                    self.run_id = self.store.current_run();
                    self.finished_failures = None;
                }
                event::Cucumber::Finished => {
                    self.finished_failures = Some(
                        self.run_id.map_or(0, |id| self.store.failures_for(id)),
                    );
                }
                _ => {}
            }
        }
        self.writer.handle_event(event, cli).await;
    }
}
impl<W, Wr: writer::Stats<W>> writer::Stats<W> for RunHooks<Wr> {
    fn passed_steps(&self) -> usize {
        self.writer.passed_steps()
    }
    fn skipped_steps(&self) -> usize {
        self.writer.skipped_steps()
    }
    fn failed_steps(&self) -> usize {
        self.writer.failed_steps()
    }
    fn retried_steps(&self) -> usize {
        self.writer.retried_steps()
    }
    fn pending_steps(&self) -> usize {
        self.writer.pending_steps()
    }

    fn pending_hooks(&self) -> usize {
        self.writer.pending_hooks()
    }

    fn pending_failures(&self) -> usize {
        self.writer.pending_failures()
    }

    fn parsing_errors(&self) -> usize {
        self.writer.parsing_errors()
    }
    fn hook_errors(&self) -> usize {
        self.writer.hook_errors() + self.run_hook_errors()
    }
    fn execution_has_failed(&self) -> bool {
        self.writer.execution_has_failed() || self.run_hook_errors() > 0
    }
}
impl<W, Value, Wr: writer::Arbitrary<W, Value>> writer::Arbitrary<W, Value>
    for RunHooks<Wr>
{
    async fn write(&mut self, value: Value) {
        self.writer.write(value).await;
    }
}
impl<Wr: writer::Normalized> writer::Normalized for RunHooks<Wr> {}
impl<Wr: writer::NonTransforming> writer::NonTransforming for RunHooks<Wr> {}
