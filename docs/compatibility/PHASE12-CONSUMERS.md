# Official consumer acceptance

This phase runs actual official JavaScript consumers over native Rust Messages
reports. It adds no Node dependency to the Rust library or runtime. Node and the
local npm packages are development-only requirements for this BDD target.

Pinned dependencies under `tests/interop`:
- `@cucumber/junit-xml-formatter` 0.15.0 (the version in Cucumber.js 13.3.0).
- `@cucumber/messages` 34.2.1 (the same protocol version).
- `@cucumber/query` 16.0.0 (within that formatter's declared dependency range).

The npm lock records the complete transitive resolution and integrity values.
Installation disables lifecycle scripts; it performs no test execution:

```sh
npm ci --prefix tests/interop --ignore-scripts --no-audit --no-fund
```

Feature-first acceptance checks a tagged Rule outline with both argument forms,
actual source example rows and native composite hook stages; then recovered and
exhausted attempts through the official final-result query and JUnit counts.
The real official formatter performs graph lookup, naming and XML rendering.
This is bounded consumer evidence, not all formatter/consumer parity.

Only after the implementation is complete:

```sh
cargo test --features output-messages --test messages_consumer -- --tags @phase12
```

Phase-end result: **2 scenarios / 8 steps passed**, exit 0, first run. The
rich report also runs the explicit Gherkin43 catalogue and docstring handler
policy; native handlers assert the same normalized substituted argument values.
The official query resolves actual example rows, raw AST template text, combined
arguments and native composite hook identities. The official JUnit formatter
renders two passing examples and, separately, one recovered pass plus one
terminal failure from four observed attempts.

Evidence: [phase12-consumers-bdd.log](evidence/phase12-consumers-bdd.log).
No other test suite, build/check/lint, deployment or publication ran.

Sources:
- https://github.com/cucumber/cucumber-js/blob/v13.3.0/package.json
- https://github.com/cucumber/junit-xml-formatter/blob/v0.15.0/javascript/src/JUnitXmlPrinter.ts
- https://github.com/cucumber/query/blob/v16.0.0/javascript/src/Query.ts
