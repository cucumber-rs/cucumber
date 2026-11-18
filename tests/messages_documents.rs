//! Phase25 featureless source integration; all execution occurs at phase-end BDD.
use std::{io::Write as _,path::PathBuf,process::{Command,Stdio},sync::LazyLock};
use cucumber::{Parser as _,World as _,given,when,then,parser::{self,SourceStore},step::{Collection,Context},writer::Messages};
use futures::{StreamExt as _,future::LocalBoxFuture};
use regex::Regex;
use serde_json::{Value,json};
#[derive(Debug,Default,cucumber::World)]
struct Documents {
    files:Vec<(PathBuf,String)>,directory:Option<tempfile::TempDir>,input:PathBuf,
    store:SourceStore,messages:Vec<Value>,error:Option<String>,generations_ok:bool,
}
#[derive(Debug,Default,cucumber::World)]
struct Execution;
fn accepted(_: &mut Execution,_:Context)->LocalBoxFuture<'_,()> {Box::pin(async {})}
fn source(shape:&str)->String {match shape {
    "blank"=>String::new(),"whitespace"=>" \t\n\t\n".into(),
    "comments without newline"=>"# one\n  # two without final newline".into(),
    "CRLF comments"=>"# one\r\n\r\n  # two\r\n".into(),
    "arbitrary text"=>"This is not a Gherkin document\n".into(),
    "language directive"=>"# language: fr\n".into(),
    _=>panic!("unknown source shape {shape}"),
}}
fn prepare(world:&mut Documents,files:Vec<(&str,String)>) {
    let dir=tempfile::tempdir().unwrap();world.input=dir.path().canonicalize().unwrap();
    world.files=files.into_iter().map(|(name,source)|{let path=world.input.join(name);std::fs::write(&path,&source).unwrap();(path,source)}).collect();
    if world.files.len()==1 {world.input=world.files[0].0.clone();}world.directory=Some(dir);
}
#[given(expr="a {string} featureless source")]
fn empty_source(world:&mut Documents,shape:String) {prepare(world,vec![("empty.feature",source(&shape))]);}
#[given(expr="a {string} invalid source")]
fn invalid_source(world:&mut Documents,shape:String) {prepare(world,vec![("invalid.feature",source(&shape))]);}
#[given("a directory containing blank commented and ordinary feature files")]
fn mixed(world:&mut Documents) {prepare(world,vec![("blank.feature",source("blank")),("comments.feature",source("comments without newline")),("ordinary.feature","Feature: Ordinary\n  Scenario: Runs\n    Given work\n".into())]);}
async fn run(world:&mut Documents,latest:bool) {
    let mut output=Vec::new();
    let app=Execution::cucumber().steps(Collection::new().given(None,Regex::new("^work$").unwrap(),accepted));
    let app=if latest {app.with_gherkin43().expect("English latest mode")}else{app};
    app.with_source_store(world.store.clone()).with_writer(Messages::with_source_store::<Execution>(&mut output,world.store.clone()))
        .with_default_cli().run(&world.input).await;
    world.messages=String::from_utf8(output).unwrap().lines().map(|line|serde_json::from_str(line).unwrap()).collect();
}
#[when("it runs through the explicit latest Cucumber convenience and Messages")]
async fn latest_run(world:&mut Documents) {run(world,true).await;}
#[when("it runs with default parser policies and source-backed Messages")]
async fn legacy_run(world:&mut Documents) {run(world,false).await;}
fn messages<'a>(world:&'a Documents,key:&str)->Vec<&'a Value>{world.messages.iter().filter_map(|m|m.get(key)).collect()}
fn oracle(source:&str,uri:&str)->Result<Value,String> {
    let mut child=Command::new("node").args(["tests/interop/gherkin43/oracle.mjs",uri]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("pinned oracle installed");
    child.stdin.take().unwrap().write_all(source.as_bytes()).unwrap();let out=child.wait_with_output().unwrap();
    if out.status.success(){Ok(serde_json::from_slice(&out.stdout).unwrap())}else{Err(String::from_utf8(out.stderr).unwrap())}
}
#[then("its exact source and empty graph equal the pinned official parser")]
fn empty_graph(world:&mut Documents) {
    let sources=messages(world,"source");let documents=messages(world,"gherkinDocument");assert_eq!(sources.len(),1);assert_eq!(documents.len(),1);
    assert_eq!(sources[0]["data"],world.files[0].1);let uri=sources[0]["uri"].as_str().unwrap();
    let official=oracle(&world.files[0].1,uri).expect("official valid featureless input");
    assert_eq!(*documents[0],official["gherkinDocument"]);assert_eq!(official["pickles"],json!([]));
}
#[then("it has zero Pickles or test cases and a successful schema-valid run")]
fn no_execution(world:&mut Documents) {
    for key in ["pickle","testCase","testCaseStarted","testCaseFinished","testStepStarted","testStepFinished"] {assert!(messages(world,key).is_empty(),"invented {key}");}
    assert_eq!(messages(world,"testRunFinished").len(),1);assert_eq!(messages(world,"testRunFinished")[0]["success"],true);schema(world);
}
#[then("every source has exactly one document and only the ordinary feature executes")]
fn mixed_report(world:&mut Documents) {
    let sources=messages(world,"source");let documents=messages(world,"gherkinDocument");assert_eq!(sources.len(),3);assert_eq!(documents.len(),3);
    for (path,source) in &world.files {
        let uri=path.to_str().unwrap();assert_eq!(sources.iter().filter(|s|s["uri"]==uri).count(),1);
        assert_eq!(sources.iter().find(|s|s["uri"]==uri).unwrap()["data"],*source);
        let docs=documents.iter().filter(|d|d["uri"]==uri).collect::<Vec<_>>();assert_eq!(docs.len(),1);
        if !source.starts_with("Feature:") {assert_eq!(**docs[0],oracle(source,uri).unwrap()["gherkinDocument"]);}
    }
    assert_eq!(messages(world,"pickle").len(),1);assert_eq!(messages(world,"testCase").len(),1);
    assert_eq!(messages(world,"testCaseFinished").len(),1);assert_eq!(messages(world,"testStepFinished").len(),1);
    assert_eq!(messages(world,"testStepFinished")[0]["testStepResult"]["status"],"PASSED");
    assert_eq!(messages(world,"testRunFinished")[0]["success"],true);
}
#[then("all document envelopes satisfy the pinned schema")]
fn schema(world:&mut Documents) {
    static VALIDATOR:LazyLock<jsonschema::Validator>=LazyLock::new(||jsonschema::validator_for(&serde_json::from_str(include_str!("fixtures/messages/messages-34.2.1.schema.json")).unwrap()).unwrap());
    for envelope in &world.messages {let errors=VALIDATOR.iter_errors(envelope).map(|e|e.to_string()).collect::<Vec<_>>();assert!(errors.is_empty(),"{envelope}: {errors:?}");}
}
#[then("the pinned official parser and native runtime both reject the document")]
fn rejected(world:&mut Documents) {
    assert!(oracle(&world.files[0].1,world.files[0].0.to_str().unwrap()).is_err());assert_eq!(messages(world,"parseError").len(),1);
    assert!(cucumber::writer::messages::source::SourceGraph::parse_gherkin43("rejected.feature",&world.files[0].1,"en").is_err());
}
#[then("it has no empty document record and no successful run")]
fn rejected_record(world:&mut Documents) {
    assert!(world.store.empty_document(&world.files[0].0).is_none());assert!(messages(world,"gherkinDocument").is_empty());
    assert_eq!(messages(world,"testRunFinished")[0]["success"],false);schema(world);
}
#[then("the legacy report contains a parse error and no empty document")]
fn legacy_rejection(world:&mut Documents) {assert_eq!(messages(world,"parseError").len(),1);rejected_record(world);}
#[when("a latest parser with French fallback captures the document")]
async fn french_capture(world:&mut Documents) {
    let parsed=parser::Basic::new().language("fr").unwrap().with_gherkin43().unwrap().with_source_store(world.store.clone())
        .parse(world.input.clone(),Default::default()).collect::<Vec<_>>().await;
    assert!(parsed.is_empty());
}
#[then("its immutable empty record preserves exact bytes path and French fallback")]
fn french_record(world:&mut Documents) {
    let record=world.store.empty_document(&world.input).expect("empty record captured");assert_eq!(record.source(),world.files[0].1);assert_eq!(record.path(),world.input.as_path());assert_eq!(record.default_language(),"fr");
}
#[then("Feature snapshots stay absent while input bytes remain available")]
fn record_types(world:&mut Documents) {assert!(world.store.get(&world.input).is_none());assert_eq!(world.store.input(&world.input).as_deref(),Some(world.files[0].1.as_str()));}
#[given("a Basic parser configured with the legacy formal dialect")]
fn formal(_: &mut Documents) {parser::Basic::new().language("formal").unwrap();}
#[when("latest document semantics are requested")]
fn request_latest(world:&mut Documents) {world.error=Some(parser::Basic::new().language("formal").unwrap().with_gherkin43().expect_err("formal is not in official catalogue").to_string());}
#[then("the unsupported fallback is reported without silent replacement")]
fn unsupported(world:&mut Documents) {assert!(world.error.as_ref().unwrap().contains("formal"));}
async fn capture(world:&Documents)->Vec<parser::Result<cucumber::gherkin::Feature>> {
    parser::Basic::new().with_gherkin43().unwrap().with_source_store(world.store.clone()).parse(world.input.clone(),Default::default()).collect().await
}
#[when("successive captures replace it with a comment feature invalid text and missing file")]
async fn generations(world:&mut Documents) {
    assert!(capture(world).await.is_empty());let first=world.store.empty_document(&world.input).unwrap();
    std::fs::write(&world.input,"# replacement\n").unwrap();assert!(capture(world).await.is_empty());let second=world.store.empty_document(&world.input).unwrap();assert_eq!(second.source(),"# replacement\n");assert_eq!(first.source(),world.files[0].1);
    std::fs::write(&world.input,"Feature: Replacement\n").unwrap();let valid=capture(world).await;assert_eq!(valid.len(),1);assert!(valid[0].is_ok());assert!(world.store.empty_document(&world.input).is_none());assert!(world.store.get(&world.input).is_some());
    std::fs::write(&world.input,"not gherkin\n").unwrap();let invalid=capture(world).await;assert_eq!(invalid.len(),1);assert!(invalid[0].is_err());assert!(world.store.empty_document(&world.input).is_none());assert!(world.store.get(&world.input).is_none());assert_eq!(world.store.input(&world.input).as_deref(),Some("not gherkin\n"));
    std::fs::remove_file(&world.input).unwrap();let missing=capture(world).await;assert_eq!(missing.len(),1);assert!(missing[0].is_err());assert!(world.store.empty_document(&world.input).is_none());assert!(world.store.input(&world.input).is_none());
    assert_eq!(first.source(),world.files[0].1);assert_eq!(second.source(),"# replacement\n");world.generations_ok=true;
}
#[then("each current generation has the correct record type and retained empty bytes stay unchanged")]
fn generations_preserved(world:&mut Documents) {assert!(world.generations_ok);}
#[tokio::main]
async fn main(){Documents::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/messages_documents").await;}
