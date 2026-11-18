//! Phase37a: actual ordered native dispatch, pinned shuffle and retained retry slots.
use std::{collections::HashMap, convert::Infallible, fmt, io::Write as _, 
    process::{Command,Stdio}, sync::{Arc,Mutex,atomic::{AtomicUsize,Ordering}}, time::Duration};
use cucumber::{given,when,then, World as _,WriterExt as _,StatsWriter as _,
    feature::Ext as _, event, Event, parser, runner::{SchedulingOptions,ScenarioOrder,Hooks,StepHooks,RunHooks,TimeoutClock},
    step::{Collection,Context}, writer::{self,Coloring}};
use futures::{StreamExt as _,future::{BoxFuture,LocalBoxFuture}};
use tokio::sync::{Barrier,Semaphore};
use regex::Regex;
static WORLDS:AtomicUsize=AtomicUsize::new(0);
#[derive(Debug,Default)]struct Observed {
    starts:Vec<String>,finishes:Vec<String>,attempts:HashMap<String,usize>,active:usize,peak:usize,
    identities:Vec<(String,usize,String,usize,usize)>,feature_finishes:usize,errors:usize,run_hooks:usize,
    serial_overlap:bool,body_starts:Vec<String>,
}
#[derive(Debug)]struct Controls {
    pair:Barrier,delay_entered:Semaphore,delay_release:Semaphore,b_entered:Semaphore,b_release:Semaphore,
    source_entered:Semaphore,source_release:Semaphore,completion_entered:Semaphore,completion_release:Semaphore,
}
impl Default for Controls {fn default()->Self {Self{pair:Barrier::new(2),delay_entered:Semaphore::new(0),delay_release:Semaphore::new(0),b_entered:Semaphore::new(0),b_release:Semaphore::new(0),source_entered:Semaphore::new(0),source_release:Semaphore::new(0),completion_entered:Semaphore::new(0),completion_release:Semaphore::new(0)}}}
async fn permit(s:&Semaphore){s.acquire().await.unwrap().forget();}
#[derive(Debug)]struct Clock(Arc<Controls>);
impl TimeoutClock for Clock {fn sleep(&self,_:Duration)->BoxFuture<'static,()> {let gates=self.0.clone();Box::pin(async move{gates.delay_entered.add_permits(1);permit(&gates.delay_release).await;})}}
#[derive(Debug,Default)]struct Application {name:String,attempt:usize,id:usize,mode:String,observed:Option<Arc<Mutex<Observed>>>,controls:Option<Arc<Controls>>}
impl cucumber::World for Application {type Error=Infallible;async fn new()->Result<Self,Self::Error>{Ok(Self{id:WORLDS.fetch_add(1,Ordering::SeqCst),..Self::default()})}}
fn body(w:&mut Application,_:Context)->LocalBoxFuture<'_,()> {Box::pin(async move {
    let state=w.observed.as_ref().unwrap();let gates=w.controls.as_ref().unwrap();
    state.lock().unwrap().body_starts.push(w.name.clone());
    if matches!(w.mode.as_str(),"capacity"|"serial") && matches!(w.name.as_str(),"A"|"B") {gates.pair.wait().await;}
    if w.mode=="delay" && w.name=="B" {gates.b_entered.add_permits(1);permit(&gates.b_release).await;}
    if matches!(w.mode.as_str(),"retry"|"delay"|"serial retry"|"exhausted"|"legacy") && w.name=="A" && (w.attempt==0 || w.mode=="exhausted") {panic!("planned retry failure");}
})}
#[derive(Clone,Debug)]struct Recorder(Arc<Mutex<Observed>>);
impl writer::NonTransforming for Recorder{}
impl cucumber::Writer<Application> for Recorder {
    type Cli=cucumber::cli::Empty;
    async fn handle_event(&mut self,input:parser::Result<Event<event::Cucumber<Application>>>,_:&Self::Cli){
        match input {
            Err(_)=>self.0.lock().unwrap().errors+=1,
            Ok(event)=>if let event::Cucumber::Feature(_,event::Feature::Finished)=event.into_inner(){self.0.lock().unwrap().feature_finishes+=1;},
        }
    }
}
struct InputParser {items:Vec<parser::Result<cucumber::gherkin::Feature>>,gates:Option<Arc<Controls>>}
impl cucumber::Parser<()> for InputParser {
    type Cli=cucumber::cli::Empty;
    type Output=futures::stream::LocalBoxStream<'static,parser::Result<cucumber::gherkin::Feature>>;
    fn parse(self,_:(),_:Self::Cli)->Self::Output {
        futures::stream::unfold((self.items.into_iter(),0,self.gates),|(mut items,index,gates)|async move{
            if let Some(g)=&gates {
                if index==1 {g.source_entered.add_permits(1);permit(&g.source_release).await;}
                if index==2 {g.completion_entered.add_permits(1);permit(&g.completion_release).await;}
            }
            items.next().map(|item|(item,(items,index+1,gates)))
        }).boxed_local()
    }
}
#[derive(Debug,Default)]struct Report {passed:usize,failed:usize,retried:usize,errors:usize,failed_run:bool}
#[derive(Default,cucumber::World)]struct Acceptance {
    mode:String,order:String,seed:String,zero_entry:String,limit:usize,filtered:bool,source_error:bool,
    observed:Arc<Mutex<Observed>>,controls:Arc<Controls>,report:Report,expected:Vec<String>,first_order:Vec<String>,
    pending:Option<LocalBoxFuture<'static,Report>>,midpoint:bool,configuration_error:String,
}
impl fmt::Debug for Acceptance {fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{f.debug_struct("SchedulingAcceptance").field("mode",&self.mode).finish()}}
fn parse(source:&str)->cucumber::gherkin::Feature {cucumber::gherkin::Feature::parse(source,Default::default()).unwrap().expand_examples().unwrap()}
fn case(name:&str)->String{format!(" Scenario: {name}\n  Given work\n")}
fn fixtures(mode:&str)->Vec<cucumber::gherkin::Feature>{
    if mode=="empty" {return vec![];}
    if matches!(mode,"order"|"filtered"|"stream"|"error") {
        return vec![parse("Feature: First\n Scenario: first\n  Given work\n Rule: Accounts\n  Scenario Outline: row <number>\n   Given work\n   Examples:\n    | number |\n    | one |\n    | two |\n"),parse("Feature: Second\n Scenario: last\n  Given work\n Rule: Final rule\n  Scenario: final\n   Given work\n")];
    }
    let mut source="Feature: Capacity\n".to_owned();
    if mode=="legacy" {source+=&case("A");source+=" @serial\n";source+=&case("S");source+=&case("B");}
    else if mode=="serial" {source+=&case("A");source+=&case("B");source+=" @serial\n";source+=&case("C");source+=&case("D");}
    else {if mode=="serial retry"{source+=" @serial\n";}source+=&case("A");source+=&case("B");if matches!(mode,"capacity"|"delay"|"zero"){source+=&case("C");}}
    vec![parse(&source)]
}
fn baseline(features:&[cucumber::gherkin::Feature])->Vec<String>{features.iter().flat_map(|f|f.scenarios.iter().chain(f.rules.iter().flat_map(|r|&r.scenarios)).map(|s|s.name.clone())).collect()}
fn application(w:&mut Acceptance)->LocalBoxFuture<'static,Report>{
    WORLDS.store(0,Ordering::SeqCst);
    let features=fixtures(&w.mode);w.expected=baseline(&features);
    if w.filtered {w.expected.retain(|s|s!="row two" && s!="last");}
    let mut items=features.into_iter().map(Ok).collect::<Vec<_>>();
    if w.source_error {items.push(Err(cucumber::gherkin::ParseFileError::Reading{path:"broken.feature".into(),source:std::io::Error::other("intentional source error")}.into()));}
    let mode=w.mode.clone();let order=w.order.clone();let seed=w.seed.clone();let limit=if w.limit==0{1}else{w.limit};let filtered=w.filtered;
    let observed=w.observed.clone();let controls=w.controls.clone();
    Box::pin(async move {
        let before=observed.clone();let after=observed.clone();let identity=observed.clone();let run_state=observed.clone();
        let bgates=controls.clone();let bmode=mode.clone();
        let hooks=Hooks::new().before("observe admission",move|_,_,scenario,world: &mut Application|{
            let state=before.clone();let gates=bgates.clone();let mode=bmode.clone();let name=scenario.name.clone();
            Box::pin(async move{
                world.name=name.clone();world.mode=mode.clone();world.observed=Some(state.clone());world.controls=Some(gates);
                let mut state=state.lock().unwrap();let attempt=*state.attempts.entry(name.clone()).or_default();*state.attempts.get_mut(&name).unwrap()+=1;world.attempt=attempt;
                if mode=="serial" && name=="C" && state.active!=0 {state.serial_overlap=true;}
                if mode=="serial" && name=="D" {assert!(state.finishes.contains(&"C:0".to_owned()));}
                if mode=="delay" && name=="C" {assert!(state.finishes.contains(&"B:0".to_owned()),"retry must retain its reserved slot");}
                if mode=="serial retry" && name=="B" {assert!(state.finishes.contains(&"A:1".to_owned()),"serial retry is still exclusive");}
                state.active+=1;state.peak=state.peak.max(state.active);state.starts.push(format!("{name}:{attempt}"));
            })
        }).after("observe completion",move|_,_,scenario,_,world|{
            let state=after.clone();let name=scenario.name.clone();let attempt=world.as_ref().map_or(0,|w|w.attempt);
            Box::pin(async move{let mut state=state.lock().unwrap();state.active-=1;state.finishes.push(format!("{name}:{attempt}"));})
        });
        let step_hooks=StepHooks::new().before("record identities",move|scope,world: &mut Application|{
            identity.lock().unwrap().identities.push((scope.scenario.name.clone(),scope.attempt,format!("{:?}",scope.scenario_id),world.id,std::ptr::from_ref(scope.scenario.as_ref()) as usize));
            Box::pin(async{})
        });
        let global=RunHooks::new().before("observe setup",move |_|{run_state.lock().unwrap().run_hooks+=1;Box::pin(async{})});
        let writer=writer::Basic::raw(std::io::sink(),Coloring::Never,0).summarized()
            .tee::<Application,_>(Recorder(observed.clone()).discard_stats_writes()).normalized();
        let mut app=cucumber::Cucumber::<Application,_,std::path::PathBuf,_,_>::new().with_parser(InputParser{items,gates:(mode=="stream").then(||controls.clone())})
            .steps(Collection::new().given(None,Regex::new("^work$").unwrap(),body))
            .with_hooks(hooks).with_step_hooks(step_hooks).with_writer(writer).with_run_hooks(global)
            .max_concurrent_scenarios(if mode=="zero" {2}else{limit}).retries(if matches!(mode.as_str(),"retry"|"delay"|"serial retry"|"exhausted"|"legacy"){1}else{0});
        if mode!="legacy" {
            let selected=match order.as_str(){"reverse"=>ScenarioOrder::Reverse,"random"=>ScenarioOrder::Random{seed},_=>ScenarioOrder::Defined};
            let mut options=SchedulingOptions::new(selected);
            if mode=="zero" {options=options.parallel(0);}
            if matches!(mode.as_str(),"delay"|"serial retry") {options=options.with_retry_clock(Arc::new(Clock(controls.clone())));app=app.retry_after(Duration::from_secs(123));}
            app=app.scheduling_options(options);
        }
        if mode=="exhausted" {app=app.fail_fast();}
        let result=app.with_default_cli().filter_run((),move|_,_,s|!filtered || (s.name!="row two" && s.name!="last")).await;
        Report{passed:result.passed_steps(),failed:result.failed_steps(),retried:result.retried_steps(),errors:result.parsing_errors(),failed_run:result.execution_has_failed()}
    })
}
fn oracle(items:&[String],seed:&str)->Vec<String>{
    let mut child=Command::new("node").arg(concat!(env!("CARGO_MANIFEST_DIR"),"/tests/fixtures/shuffle37/oracle.mjs")).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("pinned JS oracle requires Node");
    child.stdin.take().unwrap().write_all(serde_json::json!({"items":items,"seed":seed}).to_string().as_bytes()).unwrap();
    let out=child.wait_with_output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));serde_json::from_slice(&out.stdout).unwrap()
}
fn starts(w:&Acceptance)->Vec<String>{w.observed.lock().unwrap().starts.iter().map(|s|s.rsplit_once(':').unwrap().0.to_owned()).collect()}
#[given(regex=r"^(.*)$")]
fn configure(w:&mut Acceptance,text:String){
    if text.contains("spanning two features"){w.mode="order".into();}
    else if text.starts_with("an application with selected"){w.mode="filtered".into();w.filtered=true;}
    else if text.starts_with("a source stream whose"){w.mode="stream".into();}
    else if text.contains("containing valid cases"){w.mode="error".into();w.source_error=true;}
    else if text.starts_with("three ordered concurrent"){w.mode="capacity".into();}
    else if text.starts_with("the ordered cases concurrent"){w.mode="serial".into();}
    else if text.starts_with("a first case that fails once"){w.mode="retry".into();}
    else if text.starts_with("a retrying case with"){w.mode="delay".into();}
    else if text.starts_with("a serial case that retries"){w.mode="serial retry".into();}
    else if text.starts_with("a first case that fails every"){w.mode="exhausted".into();}
    else if text.starts_with("three selected cases"){w.mode="zero".into();}
    else if text.starts_with("native zero concurrency"){w.mode="invalid".into();w.zero_entry=text.split('"').nth(1).unwrap().into();}
    else if text.contains("completes without selected"){w.mode="empty".into();}
    else if text.contains("existing legacy serial-priority"){w.mode="legacy".into();}
    else {assert_eq!(text,"a following case waiting for the only execution slot");}
}
#[when(regex=r"^(.*)$")]
async fn execute(w:&mut Acceptance,text:String){
    if text=="the application resolves its execution configuration" {
        w.configuration_error=if w.zero_entry=="CLI" {use clap::Parser as _;cucumber::cli::Opts::<cucumber::cli::Empty,cucumber::runner::basic::Cli,cucumber::cli::Empty>::try_parse_from(["bdd","--concurrency","0"]).unwrap_err().to_string()}else{
            let err=std::panic::catch_unwind(||cucumber::runner::Basic::<Application>::default().max_concurrent_scenarios(0)).unwrap_err();err.downcast_ref::<String>().cloned().or_else(||err.downcast_ref::<&str>().map(|s|(*s).to_owned())).unwrap()
        };return;
    }
    if text=="the remaining source and completion gates are released" {
        w.controls.completion_release.add_permits(1);w.report=tokio::time::timeout(Duration::from_secs(10),w.pending.take().unwrap()).await.expect("buffered scheduling deadlock");return;
    }
    w.order=if text.contains("reverse"){"reverse"}else if text.contains("random"){"random"}else{"defined"}.into();
    if w.order=="random" {w.seed=text.split('"').nth(1).unwrap().into();}
    w.limit=if text.contains("two active") {2}else{1};
    let run=application(w);
    if w.mode=="stream" {
        w.order="reverse".into();
        let controls=w.controls.clone();let observed=w.observed.clone();
        let driver=async move{permit(&controls.source_entered).await;assert_eq!(WORLDS.load(Ordering::SeqCst),0);assert_eq!(observed.lock().unwrap().run_hooks,0);controls.source_release.add_permits(1);permit(&controls.completion_entered).await;};
        futures::pin_mut!(driver);
        match tokio::time::timeout(Duration::from_secs(10),futures::future::select(driver,run)).await.expect("source producer starved") {
            futures::future::Either::Left(((),run))=>{w.pending=Some(run);w.midpoint=true;},
            futures::future::Either::Right(_)=>panic!("application completed before source completion gate"),
        }return;
    }
    if matches!(w.mode.as_str(),"delay"|"serial retry") {
        let gates=w.controls.clone();let state=w.observed.clone();let mode=w.mode.clone();
        let driver=async move {
            permit(&gates.delay_entered).await;
            if mode=="delay" {permit(&gates.b_entered).await;assert!(!state.lock().unwrap().starts.iter().any(|s|s.starts_with("C:")));gates.b_release.add_permits(1);}
            else {assert_eq!(state.lock().unwrap().starts,vec!["A:0"]);}
            gates.delay_release.add_permits(1);
        };
        let (report,())=tokio::time::timeout(Duration::from_secs(10),futures::future::join(run,driver)).await.expect("retained retry slot deadlock");w.report=report;
    }else{w.report=tokio::time::timeout(Duration::from_secs(10),run).await.expect("ordered scheduling deadlock");}
    w.first_order=starts(w);
}
#[then(regex=r"^(.*)$")]
async fn verify(w:&mut Acceptance,text:String){
    let state=w.observed.clone();
    match text.as_str(){
        "every selected case executes once with its original source identity"=>{assert_eq!(w.report.passed,w.expected.len());assert_eq!(state.lock().unwrap().identities.len(),w.expected.len());assert!(!w.report.failed_run);},
        "actual case starts match the pinned JavaScript shuffle for that seed"=>assert_eq!(starts(w),oracle(&w.expected,&w.seed)),
        "repeating the run produces the same permutation"=>{let expected=w.first_order.clone();w.observed=Arc::default();w.controls=Arc::default();let run=application(w);w.report=tokio::time::timeout(Duration::from_secs(10),run).await.unwrap();assert_eq!(starts(w),expected);},
        "its actual starts match shuffling only the selected cases"=>assert_eq!(starts(w),oracle(&w.expected,&w.seed)),
        "excluded cases construct no World and invoke no callbacks"=>{assert_eq!(WORLDS.load(Ordering::SeqCst),w.expected.len());assert_eq!(state.lock().unwrap().identities.len(),w.expected.len());},
        "the producer reaches both gates without executor starvation"=>assert!(w.midpoint),
        "no World or application hook starts before source completion"=>{assert_eq!(WORLDS.load(Ordering::SeqCst),0);assert_eq!(state.lock().unwrap().run_hooks,0);},
        "actual case starts follow the complete reverse sequence"=>{let mut expected=w.expected.clone();expected.reverse();assert_eq!(starts(w),expected);},
        "the parse error remains visible and makes the run unsuccessful"=>{assert_eq!(w.report.errors,1);assert_eq!(state.lock().unwrap().errors,1);assert!(w.report.failed_run);},
        "all valid selected cases execute in defined order"=>assert_eq!(starts(w),w.expected),
        "the first two cases enter before either is released"=>{assert_eq!(state.lock().unwrap().peak,2);assert_eq!(&starts(w)[..2],["A","B"]);},
        "the third case enters only after capacity is released"=>{assert_eq!(starts(w),vec!["A","B","C"]);assert_eq!(w.report.passed,3);},
        "observed active case count never exceeds two"|"no more than two logical cases are active"=>assert_eq!(state.lock().unwrap().peak,2),
        "A and B may overlap but C waits until both have finished"=>{assert_eq!(starts(w),vec!["A","B","C","D"]);assert_eq!(state.lock().unwrap().peak,2);assert!(!state.lock().unwrap().serial_overlap);},
        "D waits until C has finished"|"C never overlaps another case"=>{assert!(!w.report.failed_run);assert!(!state.lock().unwrap().serial_overlap);},
        "both attempts of the first case finish before the following case starts"=>assert_eq!(state.lock().unwrap().starts,vec!["A:0","A:1","B:0"]),
        "retry attempts have distinct ScenarioIds and Worlds with unchanged source identity"=>{let s=state.lock().unwrap();let ids=s.identities.iter().filter(|v|v.0=="A").collect::<Vec<_>>();assert_eq!(ids.len(),2);assert_ne!(ids[0].2,ids[1].2);assert_ne!(ids[0].3,ids[1].3);assert_eq!(ids[0].4,ids[1].4);},
        "terminal counters report two passed cases and one recovered failed attempt"=>{assert_eq!((w.report.passed,w.report.failed,w.report.retried),(2,0,1));assert!(!w.report.failed_run);},
        "the retrying case retains one slot while another case can progress"=>{assert_eq!(w.report.passed,3);assert_eq!(w.report.retried,1);assert!(!w.report.failed_run);},
        "the delayed retry starts only after its delay gate is released"=>assert_eq!(state.lock().unwrap().attempts.get("A"),Some(&2)),
        "the following case cannot enter during either serial attempt or its delay"=>assert_eq!(state.lock().unwrap().starts,vec!["A:0","A:1","B:0"]),
        "it enters after the serial case reaches its terminal result"=>{assert_eq!(w.report.passed,2);assert!(!w.report.failed_run);},
        "all permitted attempts of the first case run"=>assert_eq!(state.lock().unwrap().starts,vec!["A:0","A:1"]),
        "the following case never constructs a World or invokes callbacks"=>assert_eq!(WORLDS.load(Ordering::SeqCst),2),
        "the run reports the terminal failure without duplicate feature finishes"=>{assert!(w.report.failed_run);assert_eq!(w.report.failed,1);assert_eq!(state.lock().unwrap().feature_finishes,1);},
        "all three cases execute with an active case limit of one"=>{assert_eq!(w.report.passed,3);assert_eq!(state.lock().unwrap().peak,1);},
        "configuration rejects zero with a positive-concurrency diagnostic"=>assert!(w.configuration_error.contains("positive")),
        "no World or application hook is invoked"=>{assert!(state.lock().unwrap().starts.is_empty());assert_eq!(state.lock().unwrap().run_hooks,0);},
        "the run completes without constructing a World or dispatching a case"=>{assert_eq!(WORLDS.load(Ordering::SeqCst),0);assert_eq!(w.report.passed,0);assert!(!w.report.failed_run);},
        "the established legacy fixture dispatch and retry behavior is preserved"=>{assert_eq!(state.lock().unwrap().starts,vec!["S:0","A:0","A:1","B:0"]);assert_eq!(w.report.retried,1);assert!(!w.report.failed_run);},
        other if other.starts_with("actual case starts follow the ")=>{let mut expected=w.expected.clone();if w.order=="reverse"{expected.reverse();}assert_eq!(starts(w),expected);},
        other=>panic!("unimplemented acceptance assertion: {other}"),
    }
}
#[tokio::main(flavor="current_thread")]
async fn main(){Acceptance::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/ordered_scheduling").await;}
