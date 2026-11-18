//! Phase30b: real opt-in pending/skipped executions through nine built-in consumers.
use std::{convert::Infallible,fmt,io::{self,Write as _},process::{Command,Stdio},sync::{Arc,Mutex,atomic::{AtomicUsize,Ordering}}};
use cucumber::{given,when,then,World as _,WriterExt as _,
    execution::{Completion,ExecutionControl,WorldContext,OutcomePolicy},
    step::{Collection,Context},runner::Hooks,writer::{self,Coloring}};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;

#[derive(Clone,Default)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl fmt::Debug for Buffer{fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{f.write_str("Captured output")}}
impl io::Write for Buffer {
    fn write(&mut self,bytes:&[u8])->io::Result<usize>{self.0.lock().unwrap().extend_from_slice(bytes);Ok(bytes.len())}
    fn flush(&mut self)->io::Result<()>{Ok(())}
}
impl Buffer{fn text(&self)->String{String::from_utf8(self.0.lock().unwrap().clone()).unwrap()}}
#[derive(Debug,Default)]
struct Application{control:ExecutionControl,calls:Arc<AtomicUsize>,attempt:usize}
impl cucumber::World for Application{
    type Error=Infallible;
    async fn new()->Result<Self,Self::Error>{Ok(Self::default())}
    async fn new_with_runtime_context(context:WorldContext)->Result<Self,Self::Error>{Ok(Self{control:context.execution(),..Self::default()})}
}
fn act(world:&mut Application,context:Context)->LocalBoxFuture<'_,()>{Box::pin(async move{
    world.calls.fetch_add(1,Ordering::SeqCst);
    match context.step.value.as_str(){
        "pending"=>{world.control.current().unwrap().complete(Completion::Pending).unwrap();return;}
        "skipped"=>{world.control.current().unwrap().complete(Completion::Skipped).unwrap();return;}
        "ordinary"=>{},
        "recover"=>{if world.attempt==0 {world.control.current().unwrap().complete(Completion::Pending).unwrap();return;}},
        "unreachable"=>panic!("pending setup must suppress the body"),
        other=>panic!("unexpected body {other}"),
    }
})}
#[derive(Debug,Default)]
struct ResultView{strict:bool,failed:bool,pending_steps:usize,pending_hooks:usize,pending_failures:usize,failed_steps:usize,hook_errors:usize,passed:usize,skipped:usize,retried:usize,output:String,second:String,query:Value,calls:usize}
#[derive(Debug,Default,cucumber::World)]
struct Acceptance{consumer:String,recovery:bool,results:Vec<ResultView>}
const SOURCE:&str="Feature: Explicit outcomes\n Scenario: pending step\n  Given pending\n Scenario: skipped step\n  Given skipped\n @pending_hook\n Scenario: pending hook\n  Given unreachable\n @skipped_hook\n Scenario: skipped hook\n  Given ordinary\n";
async fn execute<Wr>(writer:Wr,sources:cucumber::parser::SourceStore,strict:bool,recovery:bool)->ResultView
where Wr:cucumber::Writer<Application>+writer::Stats<Application>+writer::Normalized,
      Wr::Cli:Default,
{
    let calls=Arc::new(AtomicUsize::new(0));let observed=calls.clone();let attempts=Arc::new(AtomicUsize::new(0));
    let hooks=Hooks::new().before("controlled setup",move|_,_,scenario,world:&mut Application|{
        world.calls=observed.clone();world.attempt=attempts.fetch_add(1,Ordering::SeqCst);Box::pin(async move{
            if scenario.tags.iter().any(|tag|tag=="pending_hook"){
                world.control.current().unwrap().complete(Completion::Pending).unwrap();
                return;
            }
        })
    }).after("controlled cleanup",move|_,_,scenario,_,world|Box::pin(async move{
        if recovery && world.as_ref().is_some_and(|world|world.attempt==0){panic!("cleanup forces a retry after pending");}
        if scenario.tags.iter().any(|tag|tag=="skipped_hook"){
            world.unwrap().control.current().unwrap().complete(Completion::Skipped).unwrap();
            return;
        }
    }));
    let directory=tempfile::tempdir().unwrap();let path=directory.path().join("control.feature");std::fs::write(&path,if recovery{"Feature: Recovered control\n Scenario: recovered\n  Given recover\n"}else{SOURCE}).unwrap();
    let result=cucumber::Cucumber::new().steps(Collection::new().given(None,Regex::new("^(pending|skipped|ordinary|unreachable|recover)$").unwrap(),act))
        .with_hooks(hooks).with_source_store(sources).outcome_policy(OutcomePolicy::CucumberJs{strict_pending:strict})
        .max_concurrent_scenarios(2).retries(2).with_writer(writer).with_default_cli().run(&path).await;
    assert_eq!(result.parsing_errors(),0);
    ResultView{strict,failed:result.execution_has_failed(),pending_steps:result.pending_steps(),pending_hooks:result.pending_hooks(),pending_failures:result.pending_failures(),
        failed_steps:result.failed_steps(),hook_errors:result.hook_errors(),passed:result.passed_steps(),skipped:result.skipped_steps(),retried:result.retried_steps(),calls:calls.load(Ordering::SeqCst),..ResultView::default()}
}
fn query(output:&str)->Value{
    let mut child=Command::new("node").arg("tests/interop/consume.mjs").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(output.as_bytes()).unwrap();let result=child.wait_with_output().unwrap();assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));serde_json::from_slice(&result.stdout).unwrap()
}
#[given(expr="an application reporting pending and skipped through {string}")]
fn choose(world:&mut Acceptance,consumer:String){world.consumer=consumer;}
#[given(expr="an application whose pending attempt retries after cleanup failure through {string}")]
fn recover(world:&mut Acceptance,consumer:String){world.consumer=consumer;world.recovery=true;}
#[when("its real execution finishes under strict and non-strict policies")]
async fn run(world:&mut Acceptance){
    for strict in [true,false]{
        let output=Buffer::default();let second=Buffer::default();let sources=cucumber::parser::SourceStore::new();
        let basic=||writer::Basic::new(output.clone(),Coloring::Never,0).summarized();
        let mut result=match world.consumer.as_str(){
            "Basic"|"Summarize"=>execute(basic(),sources,strict,world.recovery).await,
            "FailOnSkipped"=>execute(basic().fail_on_skipped(),sources,strict,world.recovery).await,
            "Repeat"=>{if strict {execute(basic().repeat_failed(),sources,strict,world.recovery).await} else {execute(basic().repeat_skipped(),sources,strict,world.recovery).await}},
            "Normalize Tee"=>{
                let combined=writer::Basic::raw(output.clone(),Coloring::Never,0).summarized()
                    .tee::<Application, _>(writer::Json::for_tee(second.clone())).normalized();
                execute(combined,sources,strict,world.recovery).await
            }
            "Messages Query"=>{
                let messages=writer::Messages::raw_with_source_store(output.clone(),sources.clone())
                    .normalized().discard_arbitrary_writes().summarized();
                execute(messages,sources,strict,world.recovery).await
            }
            "JSON"=>execute(writer::Json::new(output.clone()).discard_arbitrary_writes().summarized(),sources,strict,world.recovery).await,
            "JUnit"=>{
                let observed=writer::Basic::raw(Buffer::default(),Coloring::Never,0).summarized()
                    .tee::<Application, _>(writer::JUnit::for_tee(output.clone(),0)).normalized();
                execute(observed,sources,strict,world.recovery).await
            },
            "libtest"=>execute(writer::Libtest::new(output.clone()),sources,strict,world.recovery).await,
            other=>panic!("unknown consumer {other}"),
        };
        result.output=output.text();result.second=second.text();
        if world.consumer=="Messages Query"{result.query=query(&result.output);}
        world.results.push(result);
    }
}
fn assert_json(text:&str){
    let features:Value=serde_json::from_str(text).unwrap();let elements=features[0]["elements"].as_array().unwrap();assert_eq!(elements.len(),4);
    for(name,status)in[("pending step","pending"),("skipped step","skipped"),("pending hook","skipped"),("skipped hook","passed")]{
        let element=elements.iter().find(|e|e["name"]==name).unwrap();assert_eq!(element["steps"][0]["result"]["status"],status,"{element}");
    }
    let pending=elements.iter().find(|e|e["name"]=="pending hook").unwrap();assert_eq!(pending["before"][0]["result"]["status"],"pending");
    let skipped=elements.iter().find(|e|e["name"]=="skipped hook").unwrap();assert_eq!(skipped["after"][0]["result"]["status"],"skipped");
}
fn assert_junit(result:&ResultView){
    let text=&result.output;assert_eq!(text.matches("<testcase ").count(),4,"{text}");
    assert_eq!(text.matches("<failure").count(),if result.strict{2}else{0},"{text}");
    assert_eq!(text.matches("<skipped").count(),if result.strict{2}else{4},"{text}");
    for name in ["pending step","pending hook"]{
        let cases=Regex::new(r"(?s)<testcase\b.*?</testcase>").unwrap();let case=cases.find_iter(text).find(|m|m.as_str().contains(name)).unwrap().as_str();
        assert!(case.contains(if result.strict{"<failure"}else{"<skipped"}),"{case}");
        assert!(case.contains("[PENDING]"),"metadata must survive JUnit stdout buffering: {case}");
    }
}
fn assert_libtest(result:&ResultView){
    let messages:Vec<Value>=result.output.lines().map(|line|serde_json::from_str(line).unwrap()).collect();
    let suite=messages.iter().find(|value|value["type"]=="suite"&&value["event"]!="started").unwrap();
    assert_eq!(suite["event"],if result.strict{"failed"}else{"ok"});assert_eq!(suite["failed"],if result.strict{2}else{0});
    for name in ["pending step","pending hook"]{
        let events:Vec<_>=messages.iter().filter(|value|value["type"]=="test"&&value["name"].as_str().is_some_and(|n|n.contains(name))&&value["event"]!="started").collect();
        assert!(!events.is_empty());assert!(events.iter().all(|value|value["event"]!="ok"));
        assert!(events.iter().any(|value|value["event"]==if result.strict{"failed"}else{"ignored"}));
    }
    assert_eq!(messages.iter().filter(|value|value["type"]=="test"&&value["event"]=="ok").count(),1,"only the actual ordinary body passes");
}
#[then("the consumer preserves non-passing status and correct final failure semantics")]
fn outcomes(world:&mut Acceptance){
    assert_eq!(world.results.len(),2);
    for result in &world.results{
        assert_eq!(result.failed,result.strict,"{} {result:?}",world.consumer);
        assert_eq!((result.pending_steps,result.pending_hooks),(1,1),"{} {result:?}",world.consumer);
        assert_eq!(result.pending_failures,if result.strict{2}else{0});
        assert_eq!((result.failed_steps,result.hook_errors,result.retried),(0,0,0));
        assert_eq!((result.passed,result.skipped),(1,2));assert_eq!(result.calls,3,"controlled completion never retries");
        match world.consumer.as_str(){
            "Basic"|"Summarize"|"FailOnSkipped"|"Repeat"=>{
                let text=&result.output;assert!(text.contains("Given pending [PENDING]"),"{text}");assert!(text.contains("Given skipped [SKIPPED]"),"{text}");assert!(text.lines().any(|line|line.contains("hook [PENDING]")),"{text}");
            }
            "Normalize Tee"=>assert_json(&result.second),
            "JSON"=>assert_json(&result.output),
            "JUnit"=>assert_junit(result),
            "libtest"=>assert_libtest(result),
            "Messages Query"=>{
                assert_eq!(result.query["observedAttempts"],4);let rows=result.query["outcomes"].as_array().unwrap();assert_eq!(rows.len(),4);
                for(name,status)in[("pending step","PENDING"),("skipped step","SKIPPED"),("pending hook","PENDING"),("skipped hook","SKIPPED")]{assert_eq!(rows.iter().find(|r|r["name"]==name).unwrap()["status"],status);}
                let messages:Vec<Value>=result.output.lines().map(|line|serde_json::from_str(line).unwrap()).collect();let finished=messages.iter().find_map(|m|m.get("testRunFinished")).unwrap();assert_eq!(finished["success"],!result.strict);
            }
            _=>unreachable!(),
        }
    }
}
#[then("the consumer reports the successful final attempt without stale pending failure counts")]
fn recovered(world:&mut Acceptance){
    for result in &world.results{
        assert!(!result.failed,"{} {result:?}",world.consumer);
        assert_eq!((result.pending_steps,result.pending_hooks,result.pending_failures),(0,0,0),"{} {result:?}",world.consumer);
        assert_eq!((result.failed_steps,result.hook_errors,result.skipped),(0,0,0));assert_eq!(result.passed,1);assert_eq!(result.calls,2);
        match world.consumer.as_str(){
            "Messages Query"=>{assert_eq!(result.query["observedAttempts"],2);let rows=result.query["outcomes"].as_array().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0]["status"],"PASSED");assert_eq!(rows[0]["attempt"],1);},
            "JUnit"=>{assert_eq!(result.output.matches("<testcase ").count(),1,"{}",result.output);assert!(!result.output.contains("<failure"));assert!(!result.output.contains("<skipped"));assert!(result.output.contains("recover"));},
            "libtest"=>{let messages:Vec<Value>=result.output.lines().map(|line|serde_json::from_str(line).unwrap()).collect();let suite=messages.iter().find(|m|m["type"]=="suite"&&m["event"]!="started").unwrap();assert_eq!(suite["event"],"ok");assert_eq!(suite["failed"],0);assert_eq!(suite["passed"],1);},
            "JSON"|"Normalize Tee"=>{let text=if world.consumer=="JSON"{&result.output}else{&result.second};let data:Value=serde_json::from_str(text).unwrap();let steps=data[0]["elements"][0]["steps"].as_array().unwrap();assert_eq!(steps.first().unwrap()["result"]["status"],"pending");assert_eq!(steps.last().unwrap()["result"]["status"],"passed");},
            _=>{assert!(result.output.contains("[PENDING]"));assert!(result.output.contains("recover"));},
        }
    }
}
#[tokio::main]
async fn main(){Acceptance::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/execution_control_formats").await;}
