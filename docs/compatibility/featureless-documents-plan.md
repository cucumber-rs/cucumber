# Phase25 bounded document integration plan

Feature scenarios are authored before production changes. Add explicit
Basic::with_gherkin43 -> Result and Cucumber convenience over Basic parser/runner,
selecting current latest parser policies and official empty-background execution
without changing step-definition matching. Validate any existing fallback
language through the selected catalogue. Existing separate builders stay valid.

Native Document parsing returns Option<Feature>; path-aware Document parsing can
be added without changing Feature APIs. Valid featureless results produce no
Feature stream item. SourceStore gets separate immutable EmptyDocumentSnapshot
records (path, exact source, configured fallback language). get() remains a
successful Feature snapshot; input() remains captured read bytes. Current empty
records clear on replacement/read failure/parse failure, while retained Arcs
remain immutable. A public empty_document(path) and sorted empty_documents()
read accessor support consumers.

Source-backed Messages drains unreported empty documents before successful run
finish, emitting source and the official featureless GherkinDocument once per
URI. Never emit a fabricated Feature, Pickle, TestCase or Scenario event.
Blank/comment sources compare to the pinned official parser at phase-end BDD.
Language-directive-only source is a deliberate negative acceptance: normative
gherkin.berp requires a FeatureLine after its Language token. This will be checked
against the actual pinned official parser at the phase boundary, not assumed.

Production edits are held until parent Cargo phases22/20/21 are complete; writer
changes are coordinated separately from CI23's attachment-store/case-flush edits.
No parser/oracle execution, unit suite or premature checks are part of planning.
