use cucumber::{given, then, when, StatsWriter as _, World};

#[given("ok")]
#[when("ok")]
#[then("ok")]
fn ok(_: &mut W) -> Result<(), &'static str> {
    Ok(())
}

#[given("error")]
#[when("error")]
#[then("error")]
fn error(_: &mut W) -> Result<(), &'static str> {
    Err("error")
}

#[derive(Clone, Copy, Debug, Default, World)]
struct W;

#[tokio::test]
async fn fails() {
    let writer = W::cucumber()
        .with_default_cli()
        .run("tests/features/result")
        .await;

    assert_eq!(writer.passed_steps(), 3);
    assert_eq!(writer.skipped_steps(), 0);
    assert_eq!(writer.failed_steps(), 3);
    assert_eq!(writer.retried_steps(), 0);
    assert_eq!(writer.parsing_errors(), 0);
    assert_eq!(writer.hook_errors(), 0);
}
