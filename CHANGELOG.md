# 0.7.2 — 2020-09-14

- Enforce `UnwindSafe` on async test types

# 0.7.1 — 2020-09-09

- Fix issue with `t!` macro for unbraced blocks

# 0.7.0 — 2020-09-07

- **Breaking changes**: the macro approach provided in 0.6.x and lower has been entirely removed. It was hard to maintain and limited maintenance of the tests themselves.
- A new builder approach has been implemented.
- Support for asynchronous tests has been implemented — this is runtime agnostic.