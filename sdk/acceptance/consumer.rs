//! Real external application, copied into a separate Cargo project at phase end.
use cucumber::{World as _, step::{Collection, Context}};
use cucumber_sdk::{Extension, Sdk, SdkError};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::{Value, json};
use std::{collections::HashMap, io::Read as _, path::Path, sync::{LazyLock, Mutex}};

struct OwnedExtension { name: &'static str }
impl Extension for OwnedExtension {
    fn name(&self) -> &str { self.name }
    fn execute(&self, input: &Value) -> Result<Value, SdkError> {
        Ok(json!({"owner":"external-consumer", "input":input, "transformed":input["value"].as_i64().unwrap_or_default() * 3}))
    }
}
static ATTEMPTS: LazyLock<Mutex<HashMap<String,usize>>> = LazyLock::new(Default::default);
#[derive(Debug, Default, cucumber::World)]
struct App;
fn work(_: &mut App, ctx: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move { assert_ne!(ctx.step.value, "fail", "real application failure"); })
}
fn steps() -> Collection<App> {
    Collection::new().given(None, Regex::new("^(work|pass|fail|excluded)$").unwrap(), work)
}
fn after<'a>(_: &'a cucumber::gherkin::Feature, _: Option<&'a cucumber::gherkin::Rule>, scenario: &'a cucumber::gherkin::Scenario,
    _: &'a cucumber::event::ScenarioFinished, _: Option<&'a mut App>) -> LocalBoxFuture<'a, ()> {
    Box::pin(async move {
        let attempt = { let mut counts = ATTEMPTS.lock().unwrap(); let count = counts.entry(scenario.name.clone()).or_default(); *count += 1; *count };
        assert!(scenario.name == "recovered" && attempt > 1, "real retry after-hook failure");
    })
}
async fn messages(directory: &Path) -> Value {
    std::fs::create_dir_all(directory).unwrap();
    let retries = directory.join("retries.feature");
    let outline = directory.join("outline.feature");
    let passing = directory.join("passing.feature");
    std::fs::write(&retries,"Feature: Retries\n Scenario: recovered\n  Given work\n Scenario: exhausted\n  Given work\n").unwrap();
    std::fs::write(&outline,"Feature: Rows\n Rule: resources\n  Scenario Outline: result <outcome>\n   Given <outcome>\n   Examples: selected\n    | outcome |\n    | fail |\n    | pass |\n    | fail |\n    | excluded |\n").unwrap();
    std::fs::write(&passing,"Feature: Passing\n Scenario: first\n  Given pass\n Scenario: second\n  Given pass\n").unwrap();
    let mut retry_output = Vec::new();
    App::cucumber().steps(steps()).after(after).retries(1).max_concurrent_scenarios(2)
        .with_messages(&mut retry_output).with_default_cli().run(&retries).await;
    let mut outline_output = Vec::new();
    App::cucumber().steps(steps()).with_gherkin43().unwrap().max_concurrent_scenarios(3)
        .with_messages(&mut outline_output).with_default_cli().filter_run(&outline, |_,_,s| !s.name.contains("excluded")).await;
    let mut passing_output = Vec::new();
    App::cucumber().steps(steps()).with_messages(&mut passing_output).with_default_cli().run(&passing).await;
    let mut result = serde_json::Map::new();
    for (name, bytes, source) in [("retries",retry_output,retries),("outline",outline_output,outline),("passing",passing_output,passing)] {
        let path = directory.join(format!("{name}.ndjson")); std::fs::write(&path, bytes).unwrap();
        result.insert(name.into(), json!({"messages":path,"source":source}));
    }
    Value::Object(result)
}
fn main() {
    let args:Vec<_> = std::env::args().collect();
    let mut sdk = Sdk::new();
    let value = if args.get(1).map(String::as_str) == Some("--extension") {
        sdk.register_extension(OwnedExtension{name:"consumer.triple"}).unwrap();
        let output = sdk.execute_extension("consumer.triple", &json!({"value":7})).unwrap();
        let duplicate = sdk.register_extension(OwnedExtension{name:"consumer.triple"}).is_err();
        let reserved = ["core.capabilities","core.compile","core.rerun"].map(|name| sdk.register_extension(OwnedExtension{name}).is_err());
        json!({"result":output,"duplicateRejected":duplicate,"reservedRejected":reserved,
            "capabilities":serde_json::from_str::<Value>(&sdk.request(r#"{"version":1,"operation":"capabilities","input":{}}"#)).unwrap()})
    } else if args.get(1).map(String::as_str) == Some("--selection") {
        let mut input=String::new(); std::io::stdin().read_to_string(&mut input).unwrap();
        let mut collector = cucumber::writer::messages::rerun::Collector::new();
        for line in input.lines() { collector.ingest(&serde_json::from_str::<Value>(line).unwrap()).unwrap(); }
        let plan = collector.plan().unwrap();
        let mut calls = 0;
        let selection = plan.local_selection(|uri| { calls += 1; Ok::<_,String>(std::path::PathBuf::from(uri)) }).unwrap();
        json!({"selectionIsNone":selection.is_none(),"resolverCalls":calls})
    } else if args.get(1).map(String::as_str) == Some("--messages") {
        futures::executor::block_on(messages(Path::new(args.get(2).expect("output directory"))))
    } else {
        let mut input=String::new(); std::io::stdin().read_to_string(&mut input).unwrap();
        let requests: Value=serde_json::from_str(&input).unwrap();
        let responses:Vec<Value>=requests["requests"].as_array().unwrap().iter()
            .map(|request|serde_json::from_str(&sdk.request(request.as_str().unwrap())).unwrap()).collect();
        json!({"responses":responses,"executable":std::env::current_exe().unwrap(),"manifestDir":env!("CARGO_MANIFEST_DIR")})
    };
    println!("{}",serde_json::to_string(&value).unwrap());
}
