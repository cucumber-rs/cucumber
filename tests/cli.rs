use std::{convert::Infallible, panic::AssertUnwindSafe};

use async_trait::async_trait;
use clap::Parser;
use cucumber::{
    cli::{self, Args},
    given, WorldInit,
};
use futures::FutureExt as _;

#[derive(Args)]
struct CustomCli {
    #[clap(subcommand)]
    command: Option<SubCommand>,
}

#[derive(clap::Subcommand)]
pub enum SubCommand {
    Smoke(Smoke),
}

#[derive(Args)]
pub struct Smoke {
    #[clap(long)]
    report_name: String,
}

#[derive(Clone, Copy, Debug, WorldInit)]
struct World;

#[async_trait(?Send)]
impl cucumber::World for World {
    type Error = Infallible;

    async fn new() -> Result<Self, Self::Error> {
        Ok(World)
    }
}

#[given("an invalid step")]
fn invalid_step(_world: &mut World) {
    assert!(false);
}

#[tokio::test]
// This test uses a subcommand with the global option --tags to filter
// on two failing tests and verifies that the error output contains
// 2 failing steps.
async fn tags_option_filter_all_with_subcommand() {
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

#[tokio::test]
// This test uses a subcommand with the global option --tags to filter
// on one failing test and verifies that the error output contains
// 1 failing step.
async fn tags_option_filter_scenario1_with_subcommand() {
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

#[tokio::test]
// This test verifies that the global option --tags is still available
// without subcommands and that the error output contains 1 failing step.
async fn tags_option_filter_scenario1_without_subcommand() {
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
