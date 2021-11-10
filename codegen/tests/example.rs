use std::{fs, io, panic::AssertUnwindSafe, time::Duration};

use async_trait::async_trait;
use cucumber::{gherkin::Step, given, then, when, World, WorldInit};
use futures::FutureExt as _;
use tempfile::TempDir;
use tokio::time;

#[derive(Debug, WorldInit)]
pub struct MyWorld {
    foo: i32,
    dir: TempDir,
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = io::Error;

    async fn new() -> Result<Self, Self::Error> {
        Ok(Self {
            foo: 0,
            dir: TempDir::new()?,
        })
    }
}

#[given("non-regex")]
fn test_non_regex_sync(w: &mut MyWorld) {
    w.foo += 1;
}

#[given("non-regex async")]
async fn test_non_regex_async(w: &mut MyWorld, #[step] ctx: &Step) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(ctx.value, "non-regex async");

    w.foo += 1;
}

#[given(regex = r"(\S+) is (\d+)")]
#[when(expression = r"{word} is {int}")]
async fn test_regex_async(
    w: &mut MyWorld,
    step: String,
    #[step] ctx: &Step,
    num: usize,
) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(step, "foo");
    assert_eq!(num, 0);
    assert_eq!(ctx.value, "foo is 0");

    w.foo += 1;
}

#[given(regex = r"(\S+) is sync (\d+)")]
fn test_regex_sync_slice(w: &mut MyWorld, step: &Step, matches: &[String]) {
    assert_eq!(matches[0], "foo");
    assert_eq!(matches[1].parse::<usize>().unwrap(), 0);
    assert_eq!(step.value, "foo is sync 0");

    w.foo += 1;
}

#[when(regex = r#"^I write "(\S+)" to '([^'\s]+)'$"#)]
fn test_return_result_write(
    w: &mut MyWorld,
    what: String,
    filename: String,
) -> io::Result<()> {
    let mut path = w.dir.path().to_path_buf();
    path.push(filename);
    fs::write(path, what)
}

#[then(expr = "the file {string} should contain {string}")]
fn test_return_result_read(
    w: &mut MyWorld,
    filename: String,
    what: String,
) -> io::Result<()> {
    let mut path = w.dir.path().to_path_buf();
    path.push(filename.trim_matches('\''));

    assert_eq!(what.trim_matches('"'), fs::read_to_string(path)?);

    Ok(())
}

#[tokio::main]
async fn main() {
    let res = MyWorld::cucumber()
        .max_concurrent_scenarios(None)
        .fail_on_skipped()
        .run_and_exit("./tests/features");

    let err = AssertUnwindSafe(res)
        .catch_unwind()
        .await
        .expect_err("should err");
    let err = err.downcast_ref::<String>().unwrap();

    assert_eq!(err, "1 step failed");
}
