//! Phase 14 end-of-phase BDD for official outline interpolation.
use std::{io::Write as _, process::{Command,Stdio}, sync::{LazyLock,Mutex}};
use cucumber::{given,when,then,World as _,Parser as _, feature::ExampleSemantics, parser::{Basic,SourceStore},step::{Collection,Context},writer::Messages};
use futures::{StreamExt as _,future::LocalBoxFuture};
use regex::Regex;
use serde_json::{json,Value};
#[derive(Debug,Default,cucumber::World)]
struct Acceptance { source:String, template:String, path:std::path::PathBuf, directory:Option<tempfile::TempDir>, store:SourceStore, messages:Vec<Value>, oracle:Value, legacy:Option<Result<cucumber::gherkin::Feature,String>> }
#[derive(Debug,Default,cucumber::World)]
struct Application { scenario:String }
static OBSERVED:LazyLock<Mutex<Vec<Value>>>=LazyLock::new(Default::default);
fn observe(world:&mut Application,ctx:Context)->LocalBoxFuture<'_,()> {
    OBSERVED.lock().unwrap().push(json!({"name":world.scenario,"text":ctx.step.value,"docstring":ctx.step.docstring,"table":ctx.step.table.as_ref().map(|t|&t.rows)}));
    Box::pin(async {})
}
#[given(expr="an outline exercising {string} interpolation")]
fn setup(world:&mut Acceptance,case:String) {
    let (template,headers,values)=match case.as_str() {
        "unknown token"=>("<first> <unknown>","first","42"),
        "forward cascade"=>("<first>","first | second","<second> | 42"),
        "reverse cascade"=>("<first>","second | first","42 | <second>"),
        "dollar values"=>("<first>","first","$& $1 $$"),
        "spaced header"=>("<a b>","a b","42"),
        "duplicate headers"=>("<first>","first | first","42 | 43"),
        "literal regexp"=>("<a.*> <a12>","a.* | a12","42 | 43"),
        _=>panic!("unknown case")
    };
    world.template=template.into();
    world.source=format!("Feature: Interpolation\n  Rule: Shared values\n    Scenario Outline: {template}\n      Given {template}\n        | value |\n        | {template} |\n        \"\"\"text/plain\n        {template}\n        \"\"\"\n      Examples:\n        | {headers} |\n        | {values} |\n");
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("outline.feature");std::fs::write(&path,&world.source).unwrap();world.path=path.canonicalize().unwrap();world.directory=Some(dir);
}
fn official(source:&str)->Value {
    let mut child=Command::new("node").arg("tests/interop/gherkin43/oracle.mjs").arg("outline.feature").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();let output=child.wait_with_output().unwrap();
    assert!(output.status.success(),"official oracle failed: {}",String::from_utf8_lossy(&output.stderr));serde_json::from_slice(&output.stdout).unwrap()
}
#[when("it runs with Gherkin 43 example semantics")]
async fn run_official(world:&mut Acceptance) {
    OBSERVED.lock().unwrap().clear();let mut output=Vec::new();
    Application::cucumber().steps(Collection::new().given(None,Regex::new(".*").unwrap(),observe))
        .max_concurrent_scenarios(1)
        .with_docstring_semantics(cucumber::gherkin::DocStringSemantics::Gherkin43)
        .with_example_semantics(ExampleSemantics::Gherkin43)
        .with_source_store(world.store.clone())
        .before(|_,_,scenario,world| {world.scenario=scenario.name.clone();Box::pin(async {})})
        .with_writer(Messages::with_source_store(&mut output,world.store.clone())).with_default_cli().run(&world.path).await;
    world.messages=String::from_utf8(output).unwrap().lines().map(|line|serde_json::from_str(line).unwrap()).collect();
    assert_eq!(world.messages.iter().find_map(|m|m.get("testRunFinished")).unwrap()["success"],true);
    world.oracle=official(&world.source);
}
#[then("handler names, steps, tables and docstrings match the official compiler")]
fn handler(world:&mut Acceptance) {
    let pickles=world.oracle["pickles"].as_array().unwrap();assert_eq!(pickles.len(),1);let pickle=&pickles[0];let step=&pickle["steps"][0];
    let observed=OBSERVED.lock().unwrap();assert_eq!(observed.len(),1);
    assert_eq!(observed[0]["name"],pickle["name"]);assert_eq!(observed[0]["text"],step["text"]);
    assert_eq!(observed[0]["docstring"],step["argument"]["docString"]["content"]);
    let table=step["argument"]["dataTable"]["rows"].as_array().unwrap().iter().map(|r|r["cells"].as_array().unwrap().iter().map(|c|c["value"].clone()).collect::<Vec<_>>()).collect::<Vec<_>>();
    assert_eq!(observed[0]["table"],json!(table));
}
fn payload(pickle:&Value)->Value { json!({"name":pickle["name"],"steps":pickle["steps"].as_array().unwrap().iter().map(|s|json!({"text":s["text"],"argument":s["argument"]})).collect::<Vec<_>>()}) }
#[then("the emitted Pickle payloads match the official compiler")]
fn pickles(world:&mut Acceptance) {
    let actual=world.messages.iter().filter_map(|m|m.get("pickle")).map(payload).collect::<Vec<_>>();
    let expected=world.oracle["pickles"].as_array().unwrap().iter().map(payload).collect::<Vec<_>>();assert_eq!(actual,expected);
}
#[then("the snapshot keeps raw templates and the selected example policy")]
fn snapshot(world:&mut Acceptance) {
    let snapshot=world.store.get(&world.path).unwrap();assert_eq!(snapshot.source(),world.source);assert_eq!(snapshot.example_semantics(),ExampleSemantics::Gherkin43);
    let scenario=&snapshot.feature().rules[0].scenarios[0];assert_eq!(scenario.name,world.template);assert_eq!(scenario.steps[0].value,world.template);assert_eq!(scenario.steps[0].docstring.as_deref(),Some(world.template.as_str()));
}
#[when("the default parser expands it")]
async fn legacy(world:&mut Acceptance) {
    let mut parsed=Basic::new().with_docstring_semantics(cucumber::gherkin::DocStringSemantics::Gherkin43).with_source_store(world.store.clone()).parse(&world.path,Default::default()).collect::<Vec<_>>().await;
    assert_eq!(parsed.len(),1);world.legacy=Some(parsed.remove(0).map_err(|e|e.to_string()));
    assert_eq!(world.store.get(&world.path).unwrap().example_semantics(),ExampleSemantics::Legacy);
}
#[then("the unknown placeholder remains a legacy expansion error")]
fn unknown(world:&mut Acceptance) {assert!(world.legacy.as_ref().unwrap().as_ref().unwrap_err().contains("<unknown>"));}
#[then("its substituted values retain the inserted later placeholder")]
fn single_pass(world:&mut Acceptance) {
    let scenario=&world.legacy.as_ref().unwrap().as_ref().unwrap().rules[0].scenarios[0];assert_eq!(scenario.name,"<second>");let step=&scenario.steps[0];assert_eq!(step.value,"<second>");assert_eq!(step.docstring.as_deref(),Some("<second>"));assert_eq!(step.table.as_ref().unwrap().rows[1][0],"<second>");
}
#[tokio::main]
async fn main(){Acceptance::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/interpolation").await;}
