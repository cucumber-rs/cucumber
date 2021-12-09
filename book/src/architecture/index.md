Architecture
============

On high level, the whole [`Cucumber`] is composed of three components:
- [`Parser`], representing a source of [feature]s ([default one][`parser::Basic`] parses `.feature` files).
- [`Runner`], executing [scenario]s of [feature]s received from a [`Parser`], and emitting [`event`]s ([default one][`runner::Basic`] executes concurrently).
- [`Writer`], outputting [`event`]s ([default one][`writer::Basic`] outputs to STDOUT).

Any of these components is replaceable. This makes [`Cucumber`] fully extensible, without a need to rewrite the whole library if it doesn't meet some exotic requirements. One could always write its own component, satisfying the needs, and use it. Imagine the situation, where [feature]s are sourced from distributed queue (like [Kafka]), then executed by a cluster of external workers (like [Kubernetes `Job`s][1]), and, finally, results are emitted to different reporting systems by network. All this possible by introducing custom components, capable of doing that, without a need to change the framework.

To feel a little bit of its taste, we will write some trivial implementations of each component in subchapters below. 

1. [Custom `Parser`](parser.md)
2. [Custom `Runner`](runner.md)
3. [Custom `Writer`](writer.md)




[`Cucumber`]: https://docs.rs/cucumber/*/cucumber/struct.Cucumber.html
[`event`]: https://docs.rs/cucumber/*/cucumber/event/index.html
[`Parser`]: https://docs.rs/cucumber/*/cucumber/trait.Parser.html
[`parser::Basic`]: https://docs.rs/cucumber/*/cucumber/parser/struct.Basic.html
[`Runner`]: https://docs.rs/cucumber/*/cucumber/trait.Runner.html
[`runner::Basic`]: https://docs.rs/cucumber/*/cucumber/runner/struct.Basic.html
[`Writer`]: https://docs.rs/cucumber/*/cucumber/trait.Writer.html
[`writer::Basic`]: https://docs.rs/cucumber/*/cucumber/writer/struct.Basic.html
[feature]: https://cucumber.io/docs/gherkin/reference#feature
[Kafka]: https://kafka.apache.org
[scenario]: https://cucumber.io/docs/gherkin/reference#example
[1]: https://kubernetes.io/docs/concepts/workloads/controllers/job
