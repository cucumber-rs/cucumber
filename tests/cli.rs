use std::{env, panic::AssertUnwindSafe};

use clap::Parser;
use cucumber::{World as _, cli, given};
use futures::FutureExt as _;
use serial_test::{parallel, serial};

#[derive(cli::Args)]
struct CustomCli {
    #[command(subcommand)]
    command: Option<SubCommand>,
}

#[derive(clap::Subcommand)]
enum SubCommand {
    Smoke(Smoke),
}

#[derive(cli::Args)]
struct Smoke {
    #[arg(long)]
    report_name: String,
}

#[derive(Clone, Copy, Debug, Default, cucumber::World)]
struct World;

#[given("an invalid step")]
fn invalid_step(_world: &mut World) {
    assert!(false);
}

#[test]
fn parses_shard() {
    let shard = "2/3".parse::<cli::Shard>().expect("invalid shard");

    assert_eq!(shard.index(), 2);
    assert_eq!(shard.total(), 3);
}

#[test]
fn rejects_invalid_shards() {
    for shard in ["", "1", "1/", "/1", "0/1", "1/0", "2/1", "1/2/3"] {
        assert!(
            shard.parse::<cli::Shard>().is_err(),
            "accepted invalid shard: {shard}",
        );
        assert!(
            cli::Opts::<cli::Empty, cli::Empty, cli::Empty>::try_parse_from([
                "test", "--shard", shard,
            ])
            .is_err(),
            "CLI accepted invalid shard: {shard}",
        );
    }
}

// This test uses a subcommand with the global option `--tags` to filter on two
// failing tests and verifies that the error output contains 2 failing steps.
#[tokio::test]
#[parallel]
async fn tags_option_filters_all_scenarios_with_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from([
        "test",
        "smoke",
        r#"--report-name="smoke.report""#,
        "--tags=@all",
    ])
    .expect("Invalid command line");

    let res =
        World::cucumber().with_cli(cli).run_and_exit("tests/features/cli");

    let err =
        AssertUnwindSafe(res).catch_unwind().await.expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "2 steps failed");
}

// This test uses a subcommand with the global option `--tags` to filter on one
// failing test and verifies that the error output contains 1 failing step.
#[tokio::test]
#[parallel]
async fn tags_option_filters_scenario1_with_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from([
        "test",
        "smoke",
        r#"--report-name="smoke.report""#,
        "--tags=@scenario-1",
    ])
    .expect("Invalid command line");

    let res =
        World::cucumber().with_cli(cli).run_and_exit("tests/features/cli");

    let err =
        AssertUnwindSafe(res).catch_unwind().await.expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "1 step failed");
}

// This test verifies that the global option `--tags` is still available without
// subcommands and that the error output contains 1 failing step.
#[tokio::test]
#[parallel]
async fn tags_option_filters_scenario1_no_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from([
        "test",
        "--tags=@scenario-1",
    ])
    .expect("Invalid command line");

    let res =
        World::cucumber().with_cli(cli).run_and_exit("tests/features/cli");

    let err =
        AssertUnwindSafe(res).catch_unwind().await.expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "1 step failed");
}

// This test verifies that the `CUCUMBER_FILTER_TAGS` env var filters apply and
// that the error output contains 1 failing step.
#[tokio::test]
#[serial]
async fn tags_option_filters_scenario1_via_env() {
    unsafe {
        env::set_var("CUCUMBER_FILTER_TAGS", "@scenario-1");
    }

    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from(["test"])
        .expect("Invalid command line");

    let res =
        World::cucumber().with_cli(cli).run_and_exit("tests/features/cli");

    let err =
        AssertUnwindSafe(res).catch_unwind().await.expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "1 step failed");

    unsafe {
        env::remove_var("CUCUMBER_FILTER_TAGS");
    }
}
