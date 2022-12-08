use cucumber::{given, Parameter, StatsWriter as _, World};
use derive_more::FromStr;

#[derive(FromStr, Parameter)]
#[param(regex = "\\d+", name = "u64")]
struct U64(u64);

#[given(regex = "^regex: (\\d+)$")]
#[given(expr = "expression: {u64}")]
fn assert(_: &mut W, v: U64) {
    assert_eq!(v.0, 42);
}

#[derive(Clone, Copy, Debug, Default, World)]
struct W;

#[tokio::main]
async fn main() {
    let writer = W::cucumber().run("tests/features/from_str").await;
    assert_eq!(writer.passed_steps(), 2);
    assert_eq!(writer.skipped_steps(), 0);
    assert_eq!(writer.failed_steps(), 0);
    assert_eq!(writer.parsing_errors(), 0);
    assert_eq!(writer.hook_errors(), 0);
}
