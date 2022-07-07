use std::panic::AssertUnwindSafe;

use clap::Parser;
use cucumber::{cli, given, World as _};
use futures::FutureExt as _;

#[derive(cli::Args)]
struct CustomCli {
    #[clap(subcommand)]
    command: Option<SubCommand>,
}

#[derive(clap::Subcommand)]
enum SubCommand {
    Smoke(Smoke),
}

#[derive(cli::Args)]
struct Smoke {
    #[clap(long)]
    report_name: String,
}

#[derive(Clone, Copy, Debug, Default, cucumber::World)]
struct World;

#[given("an invalid step")]
fn invalid_step(_world: &mut World) {
    assert!(false);
}

// This test uses a subcommand with the global option `--tags` to filter on two
// failing tests and verifies that the error output contains 2 failing steps.
#[tokio::test]
async fn tags_option_filters_all_scenarios_with_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from(&[
        "test",
        "smoke",
        r#"--report-name="smoke.report""#,
        "--tags=@all",
    ])
    .expect("Invalid command line");

    let res = World::cucumber()
        .with_cli(cli)
        .run_and_exit("tests/features/cli");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "2 steps failed");
}

// This test uses a subcommand with the global option `--tags` to filter on one
// failing test and verifies that the error output contains 1 failing step.
#[tokio::test]
async fn tags_option_filters_scenario1_with_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from(&[
        "test",
        "smoke",
        r#"--report-name="smoke.report""#,
        "--tags=@scenario-1",
    ])
    .expect("Invalid command line");

    let res = World::cucumber()
        .with_cli(cli)
        .run_and_exit("tests/features/cli");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "1 step failed");
}

// This test verifies that the global option `--tags` is still available without
// subcommands and that the error output contains 1 failing step.
#[tokio::test]
async fn tags_option_filters_scenario1_no_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from(&[
        "test",
        "--tags=@scenario-1",
    ])
    .expect("Invalid command line");

    let res = World::cucumber()
        .with_cli(cli)
        .run_and_exit("tests/features/cli");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "1 step failed");
}
