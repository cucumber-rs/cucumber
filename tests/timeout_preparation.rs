//! Phase30c: recording-clock acceptance of explicit callback preparation.
use std::{fmt,io,str::FromStr,sync::{Arc,LazyLock,Mutex},time::Duration};
use cucumber::{given,when,then,World as _,WriterExt as _,StatsWriter as _,
    runner::{CallbackConstruction,Timeouts,TimeoutPolicy,TimeoutClock,Hooks,StepHooks,RunHooks},
    step::{Context,Collection},writer::{self,Coloring}};
use futures::future::{BoxFuture,LocalBoxFuture};
use regex::Regex;
static TRACE:LazyLock<Mutex<Vec<String>>>=LazyLock::new(Default::default);
static VALUES:LazyLock<Mutex<Vec<String>>>=LazyLock::new(Default::default);
fn trace(value:impl Into<String>){TRACE.lock().unwrap().push(value.into());}
#[derive(Debug)]struct Scalar(u32);
impl FromStr for Scalar {type Err=std::num::ParseIntError;fn from_str(s:&str)->Result<Self,Self::Err>{trace("prepare");s.parse().map(Self)}}
#[derive(Debug)]struct Text(String);
impl FromStr for Text {type Err=std::convert::Infallible;fn from_str(s:&str)->Result<Self,Self::Err>{trace("prepare");Ok(Self(s.into()))}}
#[derive(Debug,Default,cucumber::World)]struct Application;
#[given(regex=r"^legacy regex (\d+)$")]
fn legacy_regex(_: &mut Application,value:Scalar){trace("body");VALUES.lock().unwrap().push(value.0.to_string());}
#[given(expr="legacy expr {int}")]
fn legacy_expr(_: &mut Application,value:Scalar){trace("body");VALUES.lock().unwrap().push(value.0.to_string());}
#[given(expr_js="legacy expr_js {int}")]
fn legacy_js(_: &mut Application,value:Scalar){trace("body");VALUES.lock().unwrap().push(value.0.to_string());}
#[given(regex=r"^prepared scalar (.*)$",prepare_before_timeout)]
async fn scalar(_: &mut Application,value:Scalar,step:&cucumber::gherkin::Step){
    trace("body");tokio::task::yield_now().await;assert!(step.value.starts_with("prepared scalar"));trace("borrow survived");VALUES.lock().unwrap().push(value.0.to_string());
}
#[given(expr_js="prepared quoted {string}",prepare_before_timeout)]
async fn quoted(_: &mut Application,value:Text,step:&cucumber::gherkin::Step){
    trace("body");tokio::task::yield_now().await;assert!(step.value.contains("quoted"));trace("borrow survived");VALUES.lock().unwrap().push(value.0);
}
#[given(regex=r"^prepared slice (\d+) (\d+)$",prepare_before_timeout)]
async fn slice(_: &mut Application,values:&[Scalar],step:&cucumber::gherkin::Step){
    trace("body");tokio::task::yield_now().await;assert_eq!(values.len(),2);assert_eq!(step.value,"prepared slice 4 9");trace("borrow survived");VALUES.lock().unwrap().extend(values.iter().map(|v|v.0.to_string()));
}
struct DropFuture;
impl Drop for DropFuture {fn drop(&mut self){trace("dropped");}}
fn raw(_: &mut Application,_:Context)->LocalBoxFuture<'_,()>{
    trace("factory");let guard=DropFuture;
    Box::pin(async move{let _guard=guard;trace("poll");futures::future::pending::<()>().await;})
}
fn raw_panic(_: &mut Application,_:Context)->LocalBoxFuture<'_,()>{trace("factory");panic!("intentional factory panic")}
fn plain(_: &mut Application,_:Context)->LocalBoxFuture<'_,()>{Box::pin(async{trace("body");})}
#[derive(Debug)]struct Clock{expire:bool}
impl TimeoutClock for Clock {fn sleep(&self,_:Duration)->BoxFuture<'static,()>{trace("timer");if self.expire{Box::pin(async{})}else{Box::pin(futures::future::pending())}}}
#[derive(Clone,Default)]struct Buffer(Arc<Mutex<Vec<u8>>>);
impl fmt::Debug for Buffer{fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{f.write_str("report")}}
impl io::Write for Buffer{fn write(&mut self,b:&[u8])->io::Result<usize>{self.0.lock().unwrap().extend_from_slice(b);Ok(b.len())}fn flush(&mut self)->io::Result<()>{Ok(())}}
fn factory(hook:&str)->LocalBoxFuture<'static,()>{trace(format!("factory:{hook}"));Box::pin(async{trace("hook body");})}
fn legacy_before<'a>(_: &'a cucumber::gherkin::Feature,_:Option<&'a cucumber::gherkin::Rule>,_: &'a cucumber::gherkin::Scenario,_: &'a mut Application)->LocalBoxFuture<'a,()>{factory("before scenario")}
fn legacy_after<'a>(_: &'a cucumber::gherkin::Feature,_:Option<&'a cucumber::gherkin::Rule>,_: &'a cucumber::gherkin::Scenario,_:&'a cucumber::event::ScenarioFinished,_:Option<&'a mut Application>)->LocalBoxFuture<'a,()>{factory("after scenario")}
#[derive(Debug,Default,cucumber::World)]struct Acceptance{
    mode:String,fixture:String,boundary:String,trace:Vec<String>,values:Vec<String>,report:String,failed:bool,failed_steps:usize,failed_hooks:usize,passed:usize,
}
#[given(expr="an existing {string} definition without preparation opt-in")]
fn legacy(w:&mut Acceptance,fixture:String){w.mode="legacy".into();w.fixture=fixture;}
#[given(expr="an opted-in {string} definition with before-deadline callback construction")]
fn prepared(w:&mut Acceptance,fixture:String){w.mode="prepared".into();w.fixture=fixture;}
#[given("an opted-in definition whose argument conversion fails")]
fn invalid(w:&mut Acceptance){w.mode="invalid".into();}
#[given(expr="a raw callback with {string} timeout construction policy")]
fn callback(w:&mut Acceptance,boundary:String){w.mode="raw".into();w.boundary=boundary;}
#[given("a raw callback whose before-deadline factory panics")]
fn panics(w:&mut Acceptance){w.mode="panic".into();}
#[given(expr="a {string} callback with before-deadline construction policy")]
fn hook(w:&mut Acceptance,fixture:String){w.mode="hook".into();w.fixture=fixture;}
#[given("an opted-in definition with a disabled timeout")]
fn disabled(w:&mut Acceptance){w.mode="disabled".into();}
fn clear(){TRACE.lock().unwrap().clear();VALUES.lock().unwrap().clear();}
fn collect(w:&mut Acceptance){w.trace=TRACE.lock().unwrap().clone();w.values=VALUES.lock().unwrap().clone();}
#[when("its registered callback is constructed and then first polled")]
async fn direct(w:&mut Acceptance){
    clear();let collection=Application::collection();
    let feature=cucumber::gherkin::Feature::parse(format!("Feature: Direct\n Scenario: original\n  Given legacy {} 7\n",w.fixture),Default::default()).unwrap();
    let (callback,_,_,ctx)=collection.find(&feature.scenarios[0].steps[0]).unwrap().unwrap();
    let mut world=Application;
    let future=callback(&mut world,ctx);
    assert!(TRACE.lock().unwrap().is_empty(),"legacy callback construction must stay lazy");
    future.await;collect(w);
}
#[when("its application executes with a recording timeout clock")]
async fn run(w:&mut Acceptance){
    clear();let source=match w.mode.as_str(){
        "prepared"=>match w.fixture.as_str(){"custom scalar conversion"=>"prepared scalar 42","JavaScript quoted string"=>r#"prepared quoted "a\"b""#,"slice and borrowed step"=>"prepared slice 4 9",other=>panic!("unknown fixture {other}")},
        "invalid"=>"prepared scalar invalid","disabled"=>"prepared scalar 42",_=>"raw callback",
    };
    let steps=if matches!(w.mode.as_str(),"raw"|"panic"|"hook") {
        Collection::new().given(None,Regex::new("^raw callback$").unwrap(),match w.mode.as_str(){"raw"=>raw,"panic"=>raw_panic,_=>plain})
    }else{Application::collection()};
    let policy=if w.boundary=="WithinDeadline"{CallbackConstruction::WithinDeadline}else{CallbackConstruction::BeforeDeadline};
    let timeouts=Timeouts::default().with_default(if w.mode=="disabled"{TimeoutPolicy::Disabled}else{TimeoutPolicy::After(Duration::from_secs(1))})
        .with_callback_construction(policy).with_clock(Arc::new(Clock{expire:w.mode=="raw"}));
    let mut hooks=Hooks::<Application>::new().after_with_timeout("cleanup",TimeoutPolicy::Disabled,|_,_,_,_,_|Box::pin(async{trace("cleanup");}));
    let mut step_hooks=StepHooks::<Application>::new();let mut run_hooks=RunHooks::new();
    if w.mode=="hook" {match w.fixture.as_str(){
        "before step"=>step_hooks=step_hooks.before("target",|_,_|factory("before step")),
        "after step"=>step_hooks=step_hooks.after("target",|_,_,_|factory("after step")),
        "before run"=>run_hooks=run_hooks.before("target",|_|factory("before run")),
        "after run"=>run_hooks=run_hooks.after("target",|_|factory("after run")),
        "before scenario"|"after scenario"=>{},other=>panic!("unknown hook {other}"),
    }}
    // The legacy after callback itself provides cleanup for that compatibility path.
    if w.mode=="hook" && w.fixture=="after scenario"{hooks=Hooks::new();}
    let output=Buffer::default();let writer=writer::Basic::raw(output.clone(),Coloring::Never,0).summarized().normalized();
    let mut app=cucumber::Cucumber::new().steps(steps).with_hooks(hooks).with_step_hooks(step_hooks)
        .with_writer(writer).with_run_hooks(run_hooks).with_timeouts(timeouts).max_concurrent_scenarios(1);
    if w.mode=="hook" && w.fixture=="before scenario" {app=app.before(Box::new(legacy_before as cucumber::runner::basic::BeforeHookFn<Application>));}
    if w.mode=="hook" && w.fixture=="after scenario" {app=app.after(Box::new(legacy_after as cucumber::runner::basic::AfterHookFn<Application>));}
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("prepare.feature");std::fs::write(&path,format!("Feature: Preparation\n Scenario: callback\n  Given {source}\n")).unwrap();
    let result=tokio::time::timeout(Duration::from_secs(10),app.with_default_cli().run(&path)).await.expect("preparation acceptance deadlock");
    w.failed=result.execution_has_failed();w.failed_steps=result.failed_steps();w.failed_hooks=result.hook_errors();w.passed=result.passed_steps();
    w.report=String::from_utf8(output.0.lock().unwrap().clone()).unwrap();collect(w);
}
#[then("argument conversion and the handler body first occur during polling")]
fn lazy(w:&mut Acceptance){assert_eq!(w.trace,vec!["prepare","body"]);assert_eq!(w.values,vec!["7"]);}
#[then("owned argument preparation precedes timer construction")]
fn before_timer(w:&mut Acceptance){let timer=w.trace.iter().position(|v|v=="timer").unwrap();let converted=w.trace.iter().rposition(|v|v=="prepare").unwrap();assert!(converted<timer);assert!(w.trace.iter().position(|v|v=="body").unwrap()>timer);}
#[then("its handler receives the existing converted values")]
fn values(w:&mut Acceptance){let expected=match w.fixture.as_str(){"custom scalar conversion"=>vec!["42"],"JavaScript quoted string"=>vec!["a\"b"],_=>vec!["4","9"]};assert_eq!(w.values,expected);assert_eq!(w.passed,1);assert!(!w.failed);}
#[then("borrowed arguments remain valid across suspension")]
fn borrowed(w:&mut Acceptance){assert!(w.trace.contains(&"borrow survived".into()));assert!(!w.failed);}
#[then("the existing step failure identifies argument conversion")]
fn conversion_failure(w:&mut Acceptance){assert!(w.failed);assert_eq!(w.failed_steps,1);assert!(w.report.contains("can not be parsed"));}
#[then("no body timer or handler body starts")]
fn no_body(w:&mut Acceptance){assert!(!w.trace.iter().any(|v|v=="timer"||v=="body"||v=="poll"));}
#[then("normal cleanup still runs")]
fn cleanup(w:&mut Acceptance){assert_eq!(w.trace.iter().filter(|v|*v=="cleanup").count(),1);assert_eq!(w.failed_hooks,0);}
#[then(expr="the callback factory and timer occur in {string} order")]
fn raw_order(w:&mut Acceptance,order:String){let observed=w.trace.iter().filter(|v|v.as_str()=="timer"||v.as_str()=="factory").cloned().collect::<Vec<_>>().join(" ");assert_eq!(observed,order);}
#[then("expiry still cancels its pending future and runs cleanup")]
fn cancelled(w:&mut Acceptance){assert!(w.failed);assert_eq!(w.failed_steps,1);assert!(w.report.contains("timed out"));let dropped=w.trace.iter().position(|v|v=="dropped").unwrap();let cleanup=w.trace.iter().position(|v|v=="cleanup").unwrap();assert!(dropped<cleanup);assert!(w.trace.contains(&"poll".into()));}
#[then("the existing step failure identifies the factory panic")]
fn factory_failure(w:&mut Acceptance){assert!(w.failed);assert_eq!(w.failed_steps,1);assert!(w.report.contains("intentional factory panic"));}
#[then("the selected hook factory precedes its timer")]
fn hook_order(w:&mut Acceptance){let pos=w.trace.iter().position(|v|v==&format!("factory:{}",w.fixture)).unwrap();assert_eq!(w.trace[pos+1],"timer");assert_eq!(w.trace[pos+2],"hook body");}
#[then("its established lifecycle completion and cleanup remain intact")]
fn hook_lifecycle(w:&mut Acceptance){assert!(!w.failed);assert_eq!(w.passed,1);assert_eq!(w.failed_hooks,0);if w.fixture!="after scenario"{assert!(w.trace.contains(&"cleanup".into()));}else{assert_eq!(w.trace.last().unwrap(),"hook body");}}
#[then("argument preparation and the handler run normally without any timer")]
fn no_timer(w:&mut Acceptance){assert!(!w.trace.contains(&"timer".into()));assert!(w.trace.contains(&"prepare".into()));assert!(w.trace.contains(&"body".into()));assert_eq!(w.values,vec!["42"]);assert!(!w.failed);}
#[tokio::main(flavor="current_thread")]
async fn main(){Acceptance::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/timeout_preparation").await;}
