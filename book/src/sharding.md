Sharding
========

Sharding splits scenarios between independent Cucumber runs. This is useful
when a CI system runs the same test job on multiple machines. It is different
from [`--concurrency`], which runs scenarios concurrently in one process.

Use `--shard INDEX/TOTAL` to select one shard. `INDEX` starts at 1. For
example, three independent jobs can run:

```bash
cargo test --test cucumber -- --shard 1/3
cargo test --test cucumber -- --shard 2/3
cargo test --test cucumber -- --shard 3/3
```

## GitLab CI/CD

GitLab's [`parallel`] keyword creates multiple instances of one job. GitLab
sets `CI_NODE_INDEX` to the one-based job index and `CI_NODE_TOTAL` to the
number of job instances. Pass these values directly to Cucumber:

```yaml
cucumber:
  stage: test
  parallel: 3
  script:
    - cargo test --test cucumber -- --shard "${CI_NODE_INDEX}/${CI_NODE_TOTAL}"
```

## GitHub Actions

In GitHub Actions, use a [matrix strategy] with one-based shard values. Keep
the total in the command equal to the number of values in the matrix:

```yaml
jobs:
  cucumber:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        shard: [1, 2, 3]
    steps:
      - uses: actions/checkout@v7
      - run: cargo test --test cucumber -- --shard "${{ matrix.shard }}/3"
```

Every shard must use the same feature files and filters. Otherwise, scenarios
can be omitted or executed more than once.

Cucumber applies sharding after the effective CLI or programmatic scenario
filter. CLI filters override filters configured in code. Cucumber then assigns
each remaining scenario to one shard in parser order. The default parser uses
sorted feature paths and declaration order. Rows of a [`Scenario Outline`] are
separate scenarios and can run in different shards.

Sharding balances the number of scenarios. It does not use scenario execution
times, so shards can still take different amounts of time.

Sharding can be combined with `--concurrency`. The `@serial` tag only controls
execution inside one Cucumber process; it does not serialize scenarios between
different shards.

[`--concurrency`]: cli.html
[`Scenario Outline`]: writing/scenario_outline.html
[`parallel`]: https://docs.gitlab.com/ci/yaml/#parallel
[matrix strategy]: https://docs.github.com/actions/using-jobs/using-a-matrix-for-your-jobs
