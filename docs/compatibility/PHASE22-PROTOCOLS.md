# Phase 22: explicit Messages protocol versions

`writer::messages::ProtocolVersion::{V34,V35}` selects pinned Messages34.2.1 or
35.0.0. All existing constructors retain34.2.1. `Cucumber::with_messages_protocol`
shares source snapshots and selects the version. Direct writers expose normalized
and raw protocol constructors, including source-backed variants.

The adopted envelope subset has the same schema fields under both pinned versions;
selection changes metadata. This is not a claim to emit every protocol message.
The official35.0.0 JSON schema is retained unmodified with source URL and SHA256.
Feature-first BDD checks both explicit versions and default compatibility through
rich outline arguments, composite hooks and recovered retries.

Only at the completed phase boundary:
`cargo test --features output-messages --test messages_versions -- --tags @phase22`.
Result: **3 scenarios / 12 steps passed**, exit0. Both explicit versions and
existing constructor defaults passed their pinned schemas and behavior assertions.
The initial attempt was blocked during compilation by the concurrently authored
Phase19 PEG parser action; no protocol scenario executed in that attempt. After
the complete corpus phase corrected and verified its parser, this focused target
passed without a protocol implementation correction. Evidence is retained in
`evidence/phase22-protocols-bdd.log` and
`evidence/phase22-shared-parser-compilation.log`. Protocol versions remain separate from parser/runtime policies.
