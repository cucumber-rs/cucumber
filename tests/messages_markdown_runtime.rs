//! Phase28b runtime syntax, capture and reporting acceptance; no Node runtime.
use std::{collections::{HashMap,HashSet},path::PathBuf,sync::{Arc,LazyLock,Mutex}};
use cucumber::{World as _,Parser as _,given,when,then,parser::{self,SourceFormat,SourceStore},step::{Collection,Context},writer::{Messages,messages::{ProtocolVersion,source::SourceGraph}}};
use futures::{StreamExt as _,future::LocalBoxFuture};
use regex::Regex;
use serde_json::{Value,json};
#[derive(Debug,Default,cucumber::World)]
struct Runtime {directory:Option<tempfile::TempDir>,path:PathBuf,source:String,fixture:Option<String>,language:String,store:SourceStore,messages:Vec<Value>,recorded:Vec<parser::Result<cucumber::gherkin::Feature>>,retained:Option<Arc<parser::SourceSnapshot>>,generations:bool}
#[derive(Debug,Default,cucumber::World)]
struct Execution;
static OBSERVED:LazyLock<Mutex<Vec<Value>>>=LazyLock::new(Default::default);
fn execute(_: &mut Execution,ctx:Context)->LocalBoxFuture<'_,()> {
    OBSERVED.lock().unwrap().push(json!({"text":ctx.step.value,"doc":ctx.step.docstring,"table":ctx.step.table.as_ref().map(|t|&t.rows)}));Box::pin(async {})
}
fn steps()->Collection<Execution>{Collection::new().given(None,Regex::new("^.*$").unwrap(),execute).when(None,Regex::new("^.*$").unwrap(),execute).then(None,Regex::new("^.*$").unwrap(),execute)}
struct Recorded(Vec<parser::Result<cucumber::gherkin::Feature>>);
impl<I> cucumber::Parser<I> for Recorded {type Cli=cucumber::cli::Empty;type Output=futures::stream::Iter<std::vec::IntoIter<parser::Result<cucumber::gherkin::Feature>>>;fn parse(self,_:I,_:Self::Cli)->Self::Output{futures::stream::iter(self.0)}}
fn prepare(w:&mut Runtime,name:&str,source:String){let dir=tempfile::tempdir().unwrap();w.path=dir.path().canonicalize().unwrap().join(name);std::fs::write(&w.path,&source).unwrap();w.source=source;w.directory=Some(dir);w.language="en".into();}
#[given(expr="the runtime Markdown fixture {string}")]
fn fixture(w:&mut Runtime,name:String){let source=std::fs::read_to_string(format!("tests/fixtures/gherkin43-markdown/good/{name}")).unwrap();prepare(w,&name,source);w.fixture=Some(name);}
#[given("a directory containing plain Markdown and unrelated sources")]
fn mixed(w:&mut Runtime){prepare(w,"plain.feature","Feature: Plain\n Scenario: Run\n  Given plain\n".into());let dir=w.path.parent().unwrap();std::fs::write(dir.join("markdown.feature.md"),"# Feature: Markdown\n## Scenario: Run\n* Given markdown\n").unwrap();std::fs::write(dir.join("README.md"),"# Feature: Ignored\n## Scenario: Not discovered\n* Given ignored\n").unwrap();w.path=dir.to_path_buf();}
#[given("a classic feature at a custom extension")]
fn plain_custom(w:&mut Runtime){prepare(w,"source.custom","Feature: Plain\n Scenario: Run\n  Given plain\n".into());}
#[given("Markdown at a custom extension")]
fn markdown_custom(w:&mut Runtime){prepare(w,"source.custom","# Feature: Markdown\n## Scenario: Run\n* Given markdown\n".into());}
#[given("an invalid Markdown source without a final newline")]
fn invalid(w:&mut Runtime){prepare(w,"invalid.feature.md","# Feature: Invalid\n## Scenario: Broken\n* Given payload\n````\nunterminated".into());}
#[given("an empty Markdown source")]
fn empty(w:&mut Runtime){prepare(w,"empty.feature.md",String::new());}
#[given("a French Markdown source with English-looking language prose")]
fn french(w:&mut Runtime){prepare(w,"french.feature.md","# Fonctionnalité: Café\n# language: en\n## Scénario: Boire\n* Soit café\n* Alors bon\n".into());w.language="fr".into();}
async fn run(w:&mut Runtime,format:SourceFormat){
    OBSERVED.lock().unwrap().clear();let mut output=Vec::new();
    Execution::cucumber().steps(steps()).max_concurrent_scenarios(1).language(w.language.clone()).unwrap()
        .with_source_format(format).with_source_store(w.store.clone())
        .with_writer(Messages::with_source_store_and_protocol::<Execution>(&mut output,w.store.clone(),ProtocolVersion::V35))
        .with_default_cli().run(&w.path).await;
    w.messages=String::from_utf8(output).unwrap().lines().map(|s|serde_json::from_str(s).unwrap()).collect();
}
#[when("it runs with explicit Markdown parsing and captured Messages")]
async fn markdown_run(w:&mut Runtime){run(w,SourceFormat::Markdown).await;}
#[when("it runs with Auto source format")]
async fn auto_run(w:&mut Runtime){run(w,SourceFormat::Auto).await;}
#[when("it runs with default source format")]
async fn plain_run(w:&mut Runtime){
    OBSERVED.lock().unwrap().clear();let mut output=Vec::new();
    Execution::cucumber().steps(steps()).with_messages_protocol(&mut output,ProtocolVersion::V35).with_default_cli().run(&w.path).await;
    w.messages=String::from_utf8(output).unwrap().lines().map(|s|serde_json::from_str(s).unwrap()).collect();
}
fn all<'a>(w:&'a Runtime,key:&str)->Vec<&'a Value>{w.messages.iter().filter_map(|e|e.get(key)).collect()}
#[then("all native handlers receive their Pickle text and arguments")]
fn handler_arguments(w:&mut Runtime){
    let mut expected=all(w,"pickle").into_iter().flat_map(|p|p["steps"].as_array().unwrap()).map(|s|json!({"text":s["text"],"doc":s["argument"]["docString"]["content"],"table":s["argument"]["dataTable"].get("rows").map(|rows|rows.as_array().unwrap().iter().map(|r|r["cells"].as_array().unwrap().iter().map(|c|c["value"].clone()).collect::<Vec<_>>()).collect::<Vec<_>>())})).map(|v|v.to_string()).collect::<Vec<_>>();
    let mut actual=OBSERVED.lock().unwrap().iter().map(Value::to_string).collect::<Vec<_>>();expected.sort();actual.sort();assert_eq!(actual,expected,"actual native handlers, including outline/table/doc-string payloads");
}
fn graph(w:&Runtime)->Value{json!({"gherkinDocument":all(w,"gherkinDocument")[0],"pickles":all(w,"pickle")})}
#[then("its exact Markdown source and full official graph are reported")]
fn official(w:&mut Runtime){
    let name=w.fixture.as_ref().unwrap();let base=format!("tests/fixtures/gherkin43-markdown/good/{name}");let ast:Value=serde_json::from_str(&std::fs::read_to_string(format!("{base}.ast.ndjson")).unwrap()).unwrap();let pickles=std::fs::read_to_string(format!("{base}.pickles.ndjson")).unwrap().lines().map(|s|serde_json::from_str::<Value>(s).unwrap()["pickle"].clone()).collect::<Vec<_>>();let expected=json!({"gherkinDocument":ast["gherkinDocument"],"pickles":pickles});
    let mut actual=graph(w);let mut map=HashMap::new();ids(&expected,&actual,&mut map,&mut HashSet::new());normalize(&mut actual,&map,&format!("../testdata/good/{name}"));assert_eq!(actual,expected);source_media(w,"text/x.cucumber.gherkin+markdown");
}
fn source_media(w:&Runtime,media:&str){let sources=all(w,"source");assert_eq!(sources.len(),1);assert_eq!(sources[0]["data"],w.source);assert_eq!(sources[0]["mediaType"],media);assert_eq!(sources[0]["uri"],w.path.to_str().unwrap());}
#[then("its completed run succeeds with valid execution references")]
fn references(w:&mut Runtime){
    assert!(all(w,"parseError").is_empty());assert_eq!(all(w,"testRunFinished").len(),1);assert_eq!(all(w,"testRunFinished")[0]["success"],true);
    let pickles=all(w,"pickle");let cases=all(w,"testCase");let starts=all(w,"testCaseStarted");assert_eq!(cases.len(),pickles.len());assert_eq!(starts.len(),cases.len());
    for case in cases{assert!(pickles.iter().any(|p|p["id"]==case["pickleId"]));for step in case["testSteps"].as_array().unwrap(){assert!(pickles.iter().flat_map(|p|p["steps"].as_array().unwrap()).any(|s|s["id"]==step["pickleStepId"]));}}
    for started in &starts{assert_eq!(all(w,"testCaseFinished").iter().filter(|f|f["testCaseStartedId"]==started["id"]).count(),1);}
    for finished in all(w,"testStepFinished"){assert_eq!(finished["testStepResult"]["status"],"PASSED");assert!(starts.iter().any(|s|s["id"]==finished["testCaseStartedId"]));}
}
#[then("exactly the plain and Markdown cases execute with their respective media types")]
fn mixed_report(w:&mut Runtime){assert_eq!(OBSERVED.lock().unwrap().len(),2);let sources=all(w,"source");assert_eq!(sources.len(),2);for s in sources{let markdown=s["uri"].as_str().unwrap().ends_with(".md");assert_eq!(s["mediaType"],if markdown{"text/x.cucumber.gherkin+markdown"}else{"text/x.cucumber.gherkin+plain"});assert!(!s["data"].as_str().unwrap().contains("ignored"));}references(w);}
#[then("its classic step executes and its source media type stays plain")]
fn classic(w:&mut Runtime){assert_eq!(OBSERVED.lock().unwrap()[0]["text"],"plain");source_media(w,"text/x.cucumber.gherkin+plain");references(w);}
#[then("its Markdown step executes and its source media type stays Markdown")]
fn markdown_custom_report(w:&mut Runtime){assert_eq!(OBSERVED.lock().unwrap()[0]["text"],"markdown");source_media(w,"text/x.cucumber.gherkin+markdown");references(w);}
#[then("a format failure prevents successful completion")]
fn format_error(w:&mut Runtime){assert!(OBSERVED.lock().unwrap().is_empty());assert_eq!(all(w,"parseError").len(),1);assert!(all(w,"parseError")[0]["message"].as_str().unwrap().contains("Auto source format"));assert!(all(w,"source").is_empty());assert!(w.store.input_format(&w.path).is_none());assert_eq!(all(w,"testRunFinished")[0]["success"],false);}
#[then("its parse error reports the captured Markdown bytes and original location")]
fn syntax_error(w:&mut Runtime){source_media(w,"text/x.cucumber.gherkin+markdown");assert_eq!(all(w,"parseError").len(),1);assert_eq!(all(w,"parseError")[0]["source"]["location"]["line"],4);assert_eq!(all(w,"testRunFinished")[0]["success"],false);assert!(w.store.get(&w.path).is_none());assert_eq!(w.store.input_format(&w.path),Some(SourceFormat::Markdown));}
#[given("a captured Markdown outline deleted after parsing")]
async fn deleted(w:&mut Runtime){prepare(w,"deleted.feature.md","# Feature: Original\n## Scenario Outline: case <value>\n* Given item <value>\n  | <value> |\n```text\npayload <value>\n```\n### Examples:\n  | value |\n  | --- |\n  | one |\n  | two |".into());w.recorded=parser::Basic::new().with_source_format(SourceFormat::Markdown).with_source_store(w.store.clone()).parse(w.path.clone(),Default::default()).collect().await;assert!(w.recorded.iter().all(Result::is_ok));w.retained=w.store.get(&w.path);std::fs::remove_file(&w.path).unwrap();}
#[when("its recorded native cases execute with captured Messages")]
async fn recorded(w:&mut Runtime){OBSERVED.lock().unwrap().clear();let mut output=Vec::new();Execution::cucumber::<PathBuf>().with_parser(Recorded(std::mem::take(&mut w.recorded))).steps(steps()).max_concurrent_scenarios(1).with_writer(Messages::with_source_store_and_protocol::<Execution>(&mut output,w.store.clone(),ProtocolVersion::V35)).with_default_cli().run(&w.path).await;w.messages=String::from_utf8(output).unwrap().lines().map(|s|serde_json::from_str(s).unwrap()).collect();}
#[then("the original Markdown graph and source remain available without rereading")]
fn retained(w:&mut Runtime){assert!(!w.path.exists());source_media(w,"text/x.cucumber.gherkin+markdown");let snapshot=w.retained.as_ref().unwrap();assert!(snapshot.markdown().is_some());assert_eq!(snapshot.source_format(),SourceFormat::Markdown);let expected=SourceGraph::from_snapshot(w.path.to_str().unwrap(),snapshot).unwrap();assert_eq!(graph(w),json!({"gherkinDocument":expected.gherkin_document(),"pickles":expected.pickles()}));}
#[then("an exact empty Markdown source document completes successfully")]
fn empty_report(w:&mut Runtime){source_media(w,"text/x.cucumber.gherkin+markdown");assert_eq!(all(w,"gherkinDocument")[0],&json!({"uri":w.path.to_str().unwrap(),"comments":[]}));assert!(all(w,"pickle").is_empty());assert!(all(w,"testCase").is_empty());assert_eq!(w.store.empty_document(&w.path).unwrap().source_format(),SourceFormat::Markdown);references(w);}
#[then("French steps execute and the reported graph remains French")]
fn french_report(w:&mut Runtime){assert_eq!(OBSERVED.lock().unwrap().len(),2);assert_eq!(all(w,"gherkinDocument")[0]["feature"]["language"],"fr");assert_eq!(all(w,"pickle")[0]["language"],"fr");source_media(w,"text/x.cucumber.gherkin+markdown");references(w);}
#[when("snapshots capture Markdown then plain then invalid Markdown at the same path")]
async fn generations(w:&mut Runtime){
    let store=w.store.clone();let path=w.path.clone();
    let capture=|format|parser::Basic::new().with_source_format(format).with_source_store(store.clone()).parse(path.clone(),Default::default()).collect::<Vec<_>>();
    assert!(capture(SourceFormat::Markdown).await[0].is_ok());w.retained=w.store.get(&w.path);
    std::fs::write(&w.path,"Feature: Replacement\n Scenario: Plain\n  Given plain\n").unwrap();assert!(capture(SourceFormat::Plain).await[0].is_ok());assert!(w.store.get(&w.path).unwrap().markdown().is_none());assert_eq!(w.store.input_format(&w.path),Some(SourceFormat::Plain));
    std::fs::write(&w.path,"# Feature: Invalid\n`@orphan`").unwrap();assert!(capture(SourceFormat::Markdown).await[0].is_err());assert!(w.store.get(&w.path).is_none());assert_eq!(w.store.input_format(&w.path),Some(SourceFormat::Markdown));w.generations=true;
}
#[then("retained typed Markdown metadata and bytes remain unchanged")]
fn generation_report(w:&mut Runtime){assert!(w.generations);let retained=w.retained.as_ref().unwrap();assert_eq!(retained.source(),w.source);assert_eq!(retained.source_format(),SourceFormat::Markdown);let doc=retained.markdown().unwrap();assert_eq!(doc.feature.as_ref().unwrap().scenarios[0].steps[0].doc_string.as_ref().unwrap().delimiter,"````");assert!(SourceGraph::from_snapshot(w.path.to_str().unwrap(),retained).is_ok());}
fn ids(expected:&Value,actual:&Value,map:&mut HashMap<String,String>,inverse:&mut HashSet<String>) {
    match (expected,actual) {
        (Value::Object(e),Value::Object(a))=> {
            assert_eq!(e.keys().collect::<Vec<_>>(),a.keys().collect::<Vec<_>>(),"field set differs: expected={expected} actual={actual}");
            if let Some(id)=e.get("id") {
                assert!(map.insert(a["id"].as_str().unwrap().into(),id.as_str().unwrap().into()).is_none(),"duplicate actual ID");
                assert!(inverse.insert(id.as_str().unwrap().into()),"duplicate expected ID");
            }
            for (key,value) in e { ids(value,&a[key],map,inverse); }
        },
        (Value::Array(e),Value::Array(a))=> { assert_eq!(e.len(),a.len(),"array size differs: expected={expected} actual={actual}");for (e,a) in e.iter().zip(a) { ids(e,a,map,inverse); } },
        _=>{}
    }
}
fn normalize(value:&mut Value,map:&HashMap<String,String>,uri:&str) {
    match value {
        Value::Object(object)=>for (key,value) in object {
            if key=="uri" { *value=json!(uri); }
            else if key=="id"||key.ends_with("Id") { *value=json!(map[value.as_str().unwrap()]); }
            else if key.ends_with("Ids") { for id in value.as_array_mut().unwrap() { *id=json!(map[id.as_str().unwrap()]); } }
            else { normalize(value,map,uri); }
        }, Value::Array(array)=>for item in array { normalize(item,map,uri); }, _=>{}
    }
}

#[tokio::main]
async fn main(){Runtime::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/messages_markdown_runtime").await;}
