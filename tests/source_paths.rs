//! Ordered sources exercise actual bodies and immutable source capture.
use std::{collections::HashMap, path::PathBuf, sync::{Mutex, atomic::{AtomicUsize,Ordering}}};
use cucumber::{given,when,then,World as _,StatsWriter as _,WriterExt as _,
    parser::{SourcePaths,SourceStore,SourceFormat}, runner::{SchedulingOptions,ScenarioOrder},
    step::{Collection,Context},writer};
use futures::future::LocalBoxFuture;
use regex::Regex;
static ORDER:Mutex<Vec<String>>=Mutex::new(Vec::new());
static WORLDS:AtomicUsize=AtomicUsize::new(0);
#[derive(Debug,cucumber::World)]struct Application;
impl Default for Application {fn default()->Self{WORLDS.fetch_add(1,Ordering::SeqCst);Self}}
fn body(_: &mut Application,ctx:Context)->LocalBoxFuture<'_,()>{Box::pin(async move{ORDER.lock().unwrap().push(ctx.step.value.strip_prefix("record ").unwrap().into());})}
#[derive(Debug,Default,cucumber::World)]struct Acceptance {
    directory:Option<tempfile::TempDir>, mode:String, paths:Option<SourcePaths>,store:SourceStore,
    originals:HashMap<PathBuf,String>,observed:Vec<String>,errors:usize,failed:bool,
}
#[given(expr="a source selection {string}")]
fn fixture(w:&mut Acceptance,mode:String){
    let directory=tempfile::tempdir().unwrap();let root=directory.path().canonicalize().unwrap();
    for (file,name) in [("agroup/a.feature","A"),("bgroup/b.feature","B"),("bgroup/c.feature","C")] {
        let path=root.join(file);std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let source=format!("Feature: {name}\n Scenario: selected\n  Given record {name}\n");
        std::fs::write(&path,&source).unwrap();w.originals.insert(path,source);
    }
    let a=root.join("agroup/a.feature");let b=root.join("bgroup/b.feature");let c=root.join("bgroup/c.feature");
    let p=SourcePaths::new();w.paths=Some(match mode.as_str(){
        "files"|"cli override"|"lines"=>p.path(&b).path(&a),
        "directories"=>p.path(root.join("bgroup")).path(root.join("agroup")),
        "overlap"=>p.path(&b).path(&a).path(root.join("bgroup")).path(&c).path(&b),
        "globs"=>p.glob(root.join("bgroup/*.feature").to_str().unwrap()).unwrap().glob(root.join("agroup/*.feature").to_str().unwrap()).unwrap(),
        "unicode paths"=>{
            let dir=root.join("unicode");std::fs::create_dir_all(&dir).unwrap();
            for (file,name) in [("🚀.feature","U"),(".feature","V")] {
                std::fs::write(dir.join(file),format!("Feature: {name}\n Scenario: selected\n  Given record {name}\n")).unwrap();
            }
            p.path(dir)
        },
        "empty"=>p,
        "missing"=>p.path(root.join("missing.feature")).path(&a),
        "unmatched"=>p.glob(root.join("absent*.feature").to_str().unwrap()).unwrap().path(&a),
        "invalid"=>{let path=root.join("invalid.feature");std::fs::write(&path,"Feature: bad\n Scenario: one\n  Given record X\n  | one |\n  | too | many |\n").unwrap();p.path(path).path(&a)},
        "mixed formats"=>{let path=root.join("markdown/m.feature.md");std::fs::create_dir_all(path.parent().unwrap()).unwrap();let source="# Feature: M\n## Scenario: selected\n* Given record M\n";std::fs::write(&path,source).unwrap();w.originals.insert(path,source.into());p.path(root.join("markdown")).path(&a)},
        "legacy"=>p,
        _=>panic!("unknown selection mode"),
    });
    if mode=="legacy" {w.paths=None;}
    w.directory=Some(directory);w.mode=mode;
}
#[when("the selected source plan runs")]
async fn run(w:&mut Acceptance){
    ORDER.lock().unwrap().clear();WORLDS.store(0,Ordering::SeqCst);
    let root=w.directory.as_ref().unwrap().path();let mut output=Vec::new();
    let app=Application::cucumber().steps(Collection::new().given(None,Regex::new("^record .+$").unwrap(),body))
        .with_gherkin43().unwrap().with_source_store(w.store.clone())
        .scheduling_options(SchedulingOptions::new(ScenarioOrder::Defined).parallel(0));
    let app=if let Some(paths)=&w.paths {app.with_source_paths(paths.clone())}else{app};
    let app=if w.mode=="mixed formats"{app.with_source_format(SourceFormat::Auto)}else{app};
    let app=if w.mode=="lines"{app.with_line_selectors(cucumber::filter::LineSelectors::new().lines(root.join("bgroup/b.feature"),[2]).unwrap())}else{app};
    let mut cli:cucumber::cli::Opts<cucumber::parser::basic::Cli,_,_,cucumber::cli::Empty>=Default::default();
    if w.mode=="cli override"{cli.parser.features=Some(root.join("bgroup/c.feature").to_str().unwrap().parse().unwrap());}
    let stats=app.with_writer(writer::Basic::new(&mut output,writer::Coloring::Never,writer::Verbosity::Default).summarized())
        .with_cli(cli).run(root).await;
    w.errors=stats.parsing_errors();w.failed=stats.execution_has_failed();w.observed=ORDER.lock().unwrap().clone();
    if !matches!(w.mode.as_str(),"missing"|"invalid"){assert!(!w.failed,"{}",String::from_utf8_lossy(&output));}
}
#[then(expr="observed source cases are {string}")]
fn order(w:&mut Acceptance,expected:String){let expected=expected.split(',').filter(|s|!s.is_empty()).collect::<Vec<_>>();assert_eq!(w.observed,expected);}
#[then("captured sources retain their exact original contents")]
fn sources(w:&mut Acceptance){for name in &w.observed{let (path,source)=w.originals.iter().find(|(_,s)|s.contains(&format!("record {name}\n"))).unwrap();assert_eq!(w.store.get(path).unwrap().source(),source);}}
#[then("each selected source constructs one World")]
fn once(w:&mut Acceptance){assert_eq!(WORLDS.load(Ordering::SeqCst),w.observed.len());}
#[then("one source error makes the run unsuccessful")]
fn error(w:&mut Acceptance){assert_eq!(w.errors,1);assert!(w.failed);}
#[tokio::main(flavor="current_thread")]
async fn main(){Acceptance::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/source_paths").await;}
