use std::{panic::AssertUnwindSafe, time::Duration};

use cucumber::{cli, given, then, when, writer, Parameter, World as _};
use derive_more::{Deref, FromStr};
use futures::FutureExt as _;
use tokio::time;

#[derive(cli::Args)]
struct CustomCli {
    /// Additional time to wait in before and after hooks.
    #[arg(
        long,
        default_value = "10ms",
        value_parser = humantime::parse_duration,
    )]
    pause: Duration,
}

#[tokio::main]
async fn main() {
    let cli = cli::Opts::<_, _, _, CustomCli>::parsed();

    let res = World::cucumber()
        .before(move |_, _, _, w| {
            async move {
                w.0 = 0;
                time::sleep(cli.custom.pause).await;
            }
            .boxed_local()
        })
        .after(move |_, _, _, _, _| time::sleep(cli.custom.pause).boxed_local())
        .with_writer(writer::Libtest::or_basic())
        .fail_on_skipped()
        .with_cli(cli)
        .run_and_exit("tests/features/wait");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "4 steps failed, 1 parsing error");
}

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(expr = "{u64} sec(s)")]
async fn step(world: &mut World, secs: CustomU64) {
    time::sleep(Duration::from_secs(*secs)).await;

    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}

#[derive(Deref, FromStr, Parameter)]
#[param(regex = "\\d+", name = "u64")]
struct CustomU64(u64);

#[derive(Clone, Copy, cucumber::World, Debug, Default)]
struct World(usize);
