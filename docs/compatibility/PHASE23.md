# Phase 23: typed runtime attachment foundation

Status: implemented and verified by focused phase-end BDD and affected BDD regressions.

## Adopted design

A fresh `attachment::AttachmentStore` is shared by the Basic runner and Messages
writer. The runner supplies a per-attempt `AttachmentContext` through the additive
`World::new_with_context` method; its default delegates to existing `World::new`.
Existing trait implementations and derive consumers retain their construction
behavior. The initial attachment-aware integration implements World explicitly;
a derive convenience for storing context is outside this phase.

Inside a step or scenario hook, `context.scope()` returns an immutable bound
`AttachmentHandle`. Call `handle.attach(Attachment::text(body, media_type))` or
`Attachment::bytes(bytes, media_type)`; `with_file_name` preserves an optional
caller-supplied name. The handle never retargets. Runner-owned RAII guards close
scopes at completion, unwind or cancellation; later writes return ClosedScope.
Fresh handles use the current scope. The current context itself is per attempt.

Native events remain unchanged. The shared queue records actual source/expanded
scenario identity, retry number, step ordinal or scenario-hook stage and creation
time. Messages consumes records at the completed attempt boundary and inserts
each attachment before its actual matching testStepFinished. Unknown or absent
targets must fail visibly. Text uses IDENTITY; binary encoding occurs in Messages
as BASE64, not in the runtime handle.

Before/after step hooks share their enclosing native step identity because that
is the execution test step represented by the current event protocol. Scenario
hooks target the actual native composite before/after stage. Neither approach
invents individual hook callback test steps.

## Scope and limitations

This is separate from Phase18 native tracing logs, which remain case-attempt-only.
General asynchronous activity must retain a bound handle and finish before its
scope closes; a handle does not extend the scope's lifetime.

After-scenario callbacks without a successfully constructed World cannot obtain
context through World state. Global run-hook attachments need a future hook
context API. Both remain outside this foundation. No browser integration, file
reading, streaming API, remote upload or new dependency fork is introduced.
Callers supply bytes explicitly. No publication is authorized.

## Feature-first acceptance

Nine BDD scenarios were authored before implementation. Actual steps/hooks
exercise exact text/binary media and names, deterministic concurrent scenarios,
retry attempts, rejected stale step/attempt/completed handles and the unchanged
legacy World constructor, plus lazy World creation inside the first step. The independent pinned official Query consumer
retrieves attachments by each actual TestStepFinished, including old attempts,
and independently decodes BASE64 bytes. Messages schema and reference ordering
are checked at the phase boundary only.

Evidence and exact commands are recorded below.

## Configuration

Share one store explicitly across the runner and raw Messages writer:

```rust
let attachments = cucumber::attachment::AttachmentStore::new();
let sources = cucumber::parser::SourceStore::new();
let writer = cucumber::writer::Messages::raw_with_source_store(output, sources.clone())
    .with_attachment_store(attachments.clone())
    .normalized();
cucumber::Cucumber::new()
    .steps(steps)
    .with_attachments(attachments)
    .with_source_store(sources)
    .with_writer(writer)
    .run(path)
    .await;
```

This excerpt assumes `WriterExt` is imported, `steps` identifies the application's
World and `output`/ `path` are supplied by the caller. Attachment-aware Worlds
retain the context received by their `new_with_context` override. Native logging
requires no changes to that World and remains a separate case-level capability.

## Actual phase-end evidence

The new focused command passed on its first run:

    cargo test --test messages_attachments --features output-messages -- --tags @phase23

**9 scenarios / 36 steps passed**, exit 0.
[New acceptance](evidence/phase23-attachments-bdd.log).
It exercised real Query 16.0.0 retrieval for every typed attachment, independent
binary decoding, native hook and step scope identities, original constructor
compatibility and both eager/before-hook and lazy/first-step initialization.
All attachment creation timestamps fell within their actual referenced execution
scope's started/finished timestamps. Stale writes produced no report evidence.

The first compile reported five missing public-method documentation warnings;
comments were added without changing behavior. The subsequent affected BDD
regression compile was warning-free:

    cargo test --test step_hooks --test hooks --test messages_retries --test messages_logs --test messages_backgrounds --features output-messages,tracing

- Scenario hooks: 9 scenarios / 42 steps.
- Step hooks: 10 / 51.
- Retry Messages: 6 / 40.
- Native tracing logs: 5 / 30.
- Background execution/Messages: 12 / 64.

All **42 regression scenarios / 227 steps passed**.
[Regression evidence](evidence/phase23-attachments-regressions.log).
No unit suites, generic cargo tests, lint commands or premature checks ran.
The existing optional base64 dependency is enabled by output-messages; no new
crate fork or dependency installation was needed.
