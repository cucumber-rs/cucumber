use std::{cell::RefCell, collections::HashSet, fmt, io, rc::Rc};

use cucumber::{
    StatsWriter, World as _, WriterExt as _, cli, cli::Parser as _, given,
    parser, runner, then, when, writer,
};
use futures::FutureExt as _;

#[given(regex = r"(\d+) < 10")]
#[when(regex = r"(\d+) < 10")]
#[then(regex = r"(\d+) < 10")]
fn step(_: &mut World, num: usize) {
    assert!(num < 10, "not filtered");
}

#[tokio::test]
async fn by_examples() {
    let mut output = Output::default();
    let writer = World::cucumber()
        .with_writer(
            writer::Basic::new(&mut output, writer::Coloring::Auto, 0)
                .summarized(),
        )
        .with_default_cli()
        .filter_run("tests/features/filter", |_, _, sc| {
            // Omit `Examples` rows containing numbers less than 10.
            (sc.name == "by examples")
                && (sc.examples.first().is_some_and(|example| {
                    example.table.as_ref().is_some_and(|table| {
                        table.rows.get(1).is_some_and(|cols| {
                            cols.iter().all(|v| {
                                v.parse::<usize>().is_ok_and(|num| num < 10)
                            })
                        })
                    })
                }))
        })
        .await;

    if writer.execution_has_failed() {
        panic!("some steps failed:\n{output}");
    }
}

#[tokio::test]
async fn shard_is_applied_after_programmatic_filter() {
    let cli = cli::Opts::<
        parser::basic::Cli,
        runner::basic::Cli,
        writer::basic::Cli,
    >::try_parse_from(["test", "--shard", "2/2"])
    .expect("invalid command line");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let seen_by_hook = Rc::clone(&seen);
    let mut output = Output::default();

    World::cucumber()
        .with_writer(writer::Basic::new(
            &mut output,
            writer::Coloring::Never,
            0,
        ))
        .before(move |_, _, scenario, _| {
            seen_by_hook.borrow_mut().push(scenario.name.clone());
            async {}.boxed_local()
        })
        .with_cli(cli)
        .filter_run("tests/features/filter/sharding.feature", |_, _, sc| {
            !sc.tags.iter().any(|tag| tag == "drop")
        })
        .await;

    assert_eq!(*seen.borrow(), ["outline scenario 3", "first rule scenario"],);
}

async fn scenarios_in_shard(shard: Option<&str>) -> Vec<String> {
    let mut args = vec!["test", "--tags", "@keep"];
    if let Some(shard) = shard {
        args.extend(["--shard", shard]);
    }
    let cli = cli::Opts::<
        parser::basic::Cli,
        runner::basic::Cli,
        writer::basic::Cli,
    >::try_parse_from(args)
    .expect("invalid command line");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let seen_by_hook = Rc::clone(&seen);
    let mut output = Output::default();

    World::cucumber()
        .with_writer(writer::Basic::new(
            &mut output,
            writer::Coloring::Never,
            0,
        ))
        .before(move |_, _, scenario, _| {
            seen_by_hook.borrow_mut().push(scenario.name.clone());
            async {}.boxed_local()
        })
        .with_cli(cli)
        .run("tests/features/filter/sharding.feature")
        .await;

    let scenarios = seen.borrow().clone();
    scenarios
}

#[tokio::test]
async fn shards_partition_cli_filtered_scenarios() {
    let all = scenarios_in_shard(None).await;

    assert_eq!(scenarios_in_shard(Some("1/1")).await, all);
    assert!(scenarios_in_shard(Some("6/6")).await.is_empty());

    let first = scenarios_in_shard(Some("1/2")).await;
    let second = scenarios_in_shard(Some("2/2")).await;
    let first_set = first.iter().collect::<HashSet<_>>();
    let second_set = second.iter().collect::<HashSet<_>>();
    let union = first_set.union(&second_set).copied().collect::<HashSet<_>>();
    let expected = all.iter().collect::<HashSet<_>>();

    assert!(first_set.is_disjoint(&second_set));
    assert_eq!(union, expected);
}

#[derive(Clone, Copy, Debug, Default, cucumber::World)]
struct World;

/// Deterministic output of [`writer::Basic`].
#[derive(Clone, Debug, Default)]
struct Output(Vec<u8>);

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = String::from_utf8(self.0.clone())
            .unwrap_or_else(|e| panic!("`Output` is not a string: {e}"));
        write!(f, "{o}")
    }
}
