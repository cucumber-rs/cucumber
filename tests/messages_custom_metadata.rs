//! Phase32b feature-first acceptance for explicit custom and raw metadata.
use std::{collections::BTreeSet, convert::Infallible, io::Write as _, panic::AssertUnwindSafe,
    process::{Command, Stdio}, str::FromStr, sync::{LazyLock, Mutex}};
use cucumber::{given, when, then, World as _, step::{Collection, Context, DefinitionOptions},
    support_code::{ExpressionMetadata, ParameterDescriptor, RegexMetadata}};
use futures::{FutureExt as _, future::LocalBoxFuture};
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};

#[derive(Debug, Default, cucumber::World)]
struct Report { fixture: String, output: Vec<u8>, messages: Vec<Value>, oracle: Value,
    failure: String, mapping: Option<Vec<usize>>, actual: Vec<String> }
#[derive(Debug, Default, cucumber::World)]
struct Application;
#[derive(Debug, Default, cucumber::World)]
struct Conflict;
#[derive(Debug, Default, cucumber::World)]
struct Incorrect;
static ACTUAL: LazyLock<Mutex<Vec<String>>> = LazyLock::new(Default::default);
fn record(value: impl Into<String>) { ACTUAL.lock().unwrap().push(value.into()); }
macro_rules! parsed {
    ($name:ident) => { impl FromStr for $name {type Err=Infallible;
        fn from_str(value:&str)->Result<Self,Self::Err>{Ok(Self(value.to_owned()))}} };
}
#[derive(Debug,cucumber::Parameter)]
#[param(name="color",regex="red|blue",use_for_snippets=false,prefer_for_regexp_match=true)]
struct Color(String); parsed!(Color);
#[derive(Debug,cucumber::Parameter)]
#[param(name="code",regex=r"([A-Z]+)-(\d+)",use_for_snippets=true,prefer_for_regexp_match=false)]
struct Code(String); parsed!(Code);
#[derive(Debug,cucumber::Parameter)]
#[param(name="maybe",regex="(a)?b",use_for_snippets=false,prefer_for_regexp_match=false)]
struct Maybe(String); parsed!(Maybe);
// Existing implementations need no new method or metadata flags.
#[derive(Debug)]
struct Old(String); parsed!(Old);
impl cucumber::Parameter for Old {const NAME:&'static str="old";const REGEX:&'static str="red|blue";}
#[given(expr_js="color {color}")]
fn color(_: &mut Application, value: Color){record(value.0);}
#[given(expr_js="code {code}")]
fn code(_: &mut Application, value: Code){record(value.0);}
#[given(expr_js="maybe {maybe}")]
fn maybe(_: &mut Application, value: Maybe){record(value.0);}
#[given(expr_js="😀 {int} {color}")]
fn unicode(_: &mut Application, count: i64, value: Color){record(count.to_string());record(value.0);}
#[given(expr_js="old {old}")]
fn old(_: &mut Application, value: Old){record(value.0);}
#[derive(Debug,cucumber::Parameter)]
#[param(name="same",regex="red",use_for_snippets=false,prefer_for_regexp_match=false)]
struct First(String); parsed!(First);
#[derive(Debug,cucumber::Parameter)]
#[param(name="same",regex="blue",use_for_snippets=false,prefer_for_regexp_match=false)]
struct Second(String); parsed!(Second);
#[given(expr_js="left {same}")]
fn first(_: &mut Conflict, value: First){record(value.0);}
#[given(expr_js="right {same}")]
fn second(_: &mut Conflict, value: Second){record(value.0);}
#[derive(Debug)]
struct Wrong(String); parsed!(Wrong);
impl cucumber::Parameter for Wrong {
    const NAME:&'static str="actual";const REGEX:&'static str="ok";
    fn metadata()->Option<ParameterDescriptor>{Some(ParameterDescriptor{name:"invented".into(),
        regular_expressions:vec!["different".into()],use_for_snippets:false,prefer_for_regular_expression_match:false})}
}
#[given(expr_js="wrong {actual}")]
fn wrong(_: &mut Incorrect, value: Wrong){record(value.0);}
fn raw(_: &mut Application, context: Context)->LocalBoxFuture<'_,()> {
    for (_,value) in context.matches.into_iter().skip(1){record(value);}
    Box::pin(async{})
}
fn text(fixture:&str)->&'static str {
    match fixture {
        "simple color"=>"color blue", "nested custom value"=>"code AB-42",
        "absent custom group"=>"maybe b", "Unicode prefix"=>"😀 42 blue",
        "legacy"=>"old red", "manual"=>"manual blue", "conflict"=>"left red", "incorrect"=>"wrong ok",
        "independent captures"=>"raw AB 42", "one outer argument"|"flat nested arguments"=>"raw AB-42",
        "absent and empty capture"=>"raw x", "raw Unicode prefix"=>"😀 café 42",
        "external flags"=>"READY", "replacement"=>"ready", _=>panic!("unknown fixture {fixture}"),
    }
}
#[given(expr="a custom expression fixture {string} with explicitly declared snippet and preference flags")]
fn custom_fixture(report:&mut Report, fixture:String){report.fixture=fixture;}
#[given("a custom parameter implementing only the existing name and regex contract")]
fn legacy_fixture(report:&mut Report){report.fixture="legacy".into();}
#[given("a custom expression registered through the existing explicit metadata options")]
fn manual_fixture(report:&mut Report){report.fixture="manual".into();}
#[given("two complete custom descriptors with the same name and different actual regexes")]
fn conflict_fixture(report:&mut Report){report.fixture="conflict".into();}
#[given("an explicit custom descriptor whose name or regex differs from its registered type")]
fn incorrect_fixture(report:&mut Report){report.fixture="incorrect".into();}
#[given(expr="a raw regex fixture {string} with an explicit argument capture mapping")]
fn raw_fixture(report:&mut Report, fixture:String){report.fixture=if fixture=="Unicode prefix"{"raw Unicode prefix".into()}else{fixture};}
#[given("a raw regex with a declared argument index outside its actual captures")]
fn invalid_fixture(report:&mut Report){report.fixture="invalid".into();}
#[given("explicit raw metadata paired with a matcher whose external flags change the observed match")]
fn flags_fixture(report:&mut Report){report.fixture="external flags".into();}
#[given("a raw registration with explicit capture metadata replaced through the existing insertion API")]
fn replacement_fixture(report:&mut Report){report.fixture="replacement".into();}

async fn execute(report:&mut Report) {
    ACTUAL.lock().unwrap().clear();
    let directory=tempfile::tempdir().unwrap();let path=directory.path().join("custom-metadata.feature");
    std::fs::write(&path,format!("Feature: Metadata\n  Scenario: Actual\n    Given {}\n",text(&report.fixture))).unwrap();
    let result=AssertUnwindSafe(async {
        if report.fixture=="conflict" {
            Conflict::cucumber().with_messages(&mut report.output).with_default_cli().run(&path).await;
        } else if report.fixture=="incorrect" {
            Incorrect::cucumber().with_messages(&mut report.output).with_default_cli().run(&path).await;
        } else {
            let mut steps=Application::collection();
            let raw_case=match report.fixture.as_str() {
                "independent captures"=>Some((r"^raw ([A-Z]+) ([0-9]+)$",vec![1,2])),
                "one outer argument"=>Some((r"^raw (([A-Z]+)-([0-9]+))$",vec![1])),
                "flat nested arguments"=>Some((r"^raw (([A-Z]+)-([0-9]+))$",vec![1,2,3])),
                "absent and empty capture"=>Some((r"^raw (a)?(b*)x$",vec![1,2])),
                "raw Unicode prefix"=>Some((r"^😀 café ([0-9]+)$",vec![1])),
                "external flags"|"replacement"=>Some((r"^ready$",vec![])),
                _=>None,
            };
            if let Some((pattern,mapping))=raw_case {
                let metadata=RegexMetadata::new(pattern,mapping.clone()).unwrap();
                let regex=if report.fixture=="external flags" {
                    RegexBuilder::new(pattern).case_insensitive(true).build().unwrap()
                } else {metadata.regex().clone()};
                steps=Collection::new().given_with_options(None,regex,raw,
                    DefinitionOptions::new().regular_expression(metadata));
                if report.fixture=="replacement" {
                    steps=steps.given(None,Regex::new(pattern).unwrap(),raw);
                } else {report.mapping=Some(mapping);}
            } else if report.fixture=="manual" {
                let pattern="^manual (red|blue)$";
                let metadata=ExpressionMetadata::from_generated("manual {manual_color}".into(),pattern,pattern,
                    vec![ParameterDescriptor{name:"manual_color".into(),regular_expressions:vec!["red|blue".into()],
                        use_for_snippets:false,prefer_for_regular_expression_match:false}]).unwrap();
                steps=Collection::new().given_with_options(None,Regex::new(pattern).unwrap(),raw,
                    DefinitionOptions::new().expression(metadata));
            }
            Application::cucumber().steps(steps).with_messages(&mut report.output).with_default_cli().run(&path).await;
        }
    }).catch_unwind().await;
    if let Err(error)=result {report.failure=error.downcast_ref::<String>().cloned()
        .or_else(||error.downcast_ref::<&str>().map(|s|(*s).to_owned())).unwrap_or_else(||"non-string panic".into());}
    report.actual=ACTUAL.lock().unwrap().clone();
    report.messages=String::from_utf8(report.output.clone()).unwrap().lines().map(|s|serde_json::from_str(s).unwrap()).collect();
}
#[when("its native handler and source-backed Messages reporter execute")]
#[when("their source-backed Messages reporter starts")]
async fn run(report:&mut Report) {
    execute(report).await;
    if !report.failure.is_empty(){return;}
    let mut process=Command::new("node").arg("tests/interop/expressions/custom-metadata.mjs")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    process.stdin.take().unwrap().write_all(json!({"messages":report.messages,"mapping":report.mapping}).to_string().as_bytes()).unwrap();
    let output=process.wait_with_output().unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    report.oracle=serde_json::from_slice(&output.stdout).unwrap();
}
#[when("the explicit raw metadata is constructed")]
fn invalid_mapping(report:&mut Report){report.failure=RegexMetadata::new("^one (capture)$",[2]).unwrap_err().to_string();}
fn step(report:&Report)->&Value {&report.messages.iter().find_map(|m|m.get("testCase")).unwrap()["testSteps"][0]}
fn arguments(report:&Report)->&Value {&step(report)["stepMatchArgumentsLists"][0]["stepMatchArguments"]}
fn compare(report:&Report){assert!(report.failure.is_empty(),"{}",report.failure);
    assert_eq!(arguments(report),&report.oracle["cases"][0]["matches"][0]["arguments"]);}
#[then("the original custom expression and full raw argument groups agree with the pinned official expression library")]
#[then("its complete custom descriptor and argument groups remain available")]
fn custom_groups(report:&mut Report){
    compare(report);let id=&step(report)["stepDefinitionIds"][0];
    let definition=report.messages.iter().filter_map(|m|m.get("stepDefinition")).find(|d|&d["id"]==id).unwrap();
    assert_eq!(definition["pattern"]["type"],"CUCUMBER_EXPRESSION");
    let expected=match report.fixture.as_str(){"simple color"=>"color {color}","nested custom value"=>"code {code}",
        "absent custom group"=>"maybe {maybe}","Unicode prefix"=>"😀 {int} {color}","manual"=>"manual {manual_color}",_=>unreachable!()};
    assert_eq!(definition["pattern"]["source"],expected);
}
#[then("the emitted custom descriptor retains the actual name and regex with the declared flags")]
fn descriptors(report:&mut Report){
    let expected=[("color","red|blue",false,true),("code",r"([A-Z]+)-(\d+)",true,false),("maybe","(a)?b",false,false)];
    let descriptors=report.messages.iter().filter_map(|m|m.get("parameterType")).collect::<Vec<_>>();
    let names=descriptors.iter().map(|d|d["name"].as_str().unwrap()).collect::<BTreeSet<_>>();
    assert_eq!(names,BTreeSet::from(["color","code","maybe","int"]));
    for(name,regex,snippets,prefer)in expected{
        let descriptor=descriptors.iter().find(|d|d["name"]==name).unwrap();
        assert_eq!(descriptor["regularExpressions"],json!([regex]));
        assert_eq!(descriptor["useForSnippets"],snippets);assert_eq!(descriptor["preferForRegularExpressionMatch"],prefer);
        assert!(descriptor.get("sourceReference").is_none());
    }
}
#[then("existing Rust handler conversion remains unchanged")]
fn conversion(report:&mut Report){let expected=match report.fixture.as_str(){
    "simple color"=>vec!["blue"],"nested custom value"=>vec!["AB"],"absent custom group"=>vec!["b"],
    "Unicode prefix"=>vec!["42","blue"],_=>unreachable!()};assert_eq!(report.actual,expected);}
#[then("its handler still passes and unavailable expression metadata remains omitted")]
fn omission(report:&mut Report){assert!(report.failure.is_empty(),"{}",report.failure);assert!(step(report).get("stepMatchArgumentsLists").is_none());
    assert!(report.messages.iter().any(|m|m["testStepFinished"]["testStepResult"]["status"]=="PASSED"));}
#[then("reporting identifies the descriptor conflict without a successful run-finished envelope")]
fn conflict_visible(report:&mut Report){assert!(report.failure.contains("conflicting parameter descriptors"),"{}",report.failure);
    assert!(!report.messages.iter().any(|m|m["testRunFinished"]["success"]==true));}
#[then("reporting rejects the inconsistent descriptor without fabricating metadata")]
fn mismatch_visible(report:&mut Report){assert!(report.failure.contains("metadata"),"{}",report.failure);
    assert!(!report.messages.iter().any(|m|m["testRunFinished"]["success"]==true));}
#[then("reported raw arguments follow the declared capture order and actual nested group structure")]
fn raw_mapping(report:&mut Report){compare(report);assert_eq!(arguments(report).as_array().unwrap().len(),report.mapping.as_ref().unwrap().len());}
#[then("native JavaScript indices verify their raw values and UTF-16 positions")]
fn native_ranges(report:&mut Report){compare(report);}
#[then("unknown parameter names and parameter descriptors remain omitted")]
fn unknown_types(report:&mut Report){
    for argument in arguments(report).as_array().unwrap(){assert!(argument.get("parameterTypeName").is_none());}
    assert!(!report.messages.iter().any(|m|m.get("parameterType").is_some()));
}
#[then("construction rejects the invalid capture mapping visibly")]
fn invalid_visible(report:&mut Report){assert!(report.failure.contains("capture index 2 does not exist"));}
#[then("all emitted envelopes satisfy both supported Messages schemas")]
fn schema(report:&mut Report){for source in [include_str!("fixtures/messages/messages-34.2.1.schema.json"),include_str!("fixtures/messages/messages-35.0.0.schema.json")]{
    let schema=serde_json::from_str(source).unwrap();let validator=jsonschema::validator_for(&schema).unwrap();
    for message in &report.messages{let errors=validator.iter_errors(message).map(|e|e.to_string()).collect::<Vec<_>>();assert!(errors.is_empty(),"{message}: {errors:?}");}
}}
#[tokio::main]
async fn main(){Report::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/messages_custom_metadata").await;}
