//! Phase42: compiler-facing BDD acceptance of explicit expression name policy.
//! Child execution is only the named harness=false `registration` BDD target.
use cucumber::{given, when, then, World as _};
use serde_json::{Value, json};
use std::{fs, io::Write as _, path::PathBuf, process::{Command, Stdio},
    sync::{OnceLock, atomic::{AtomicUsize, Ordering}}, time::{SystemTime, UNIX_EPOCH}};
struct Workspace { consumer: PathBuf, target: PathBuf, evidence: PathBuf, sequence: AtomicUsize }
static WORKSPACE: OnceLock<Workspace> = OnceLock::new();
#[derive(Debug, Default, cucumber::World)]
struct Acceptance { name: String, status: Option<i32>, stdout: String, stderr: String, oracle: Value, evidence: PathBuf }
#[given(expr = "a Rust custom parameter named {string}")]
fn named(w: &mut Acceptance, value: String) { w.name = value; }
fn execute(w: &mut Acceptance, mode: &str) {
    let workspace = WORKSPACE.get().expect("outer BDD workspace exists");
    let sequence = workspace.sequence.fetch_add(1, Ordering::SeqCst);
    let evidence = workspace.evidence.join(format!("{sequence:02}-{mode}"));
    fs::create_dir_all(&evidence).unwrap();
    w.evidence = evidence.clone();
    let expression = format!("value {{{}}}", w.name);
    let source = format!(r#"use cucumber::{{given, then, World as _}};
use std::{{convert::Infallible, str::FromStr}};
#[derive(Debug, Default, cucumber::World)]
struct Registration {{ seen: bool }}
#[derive(Debug, cucumber::Parameter)]
#[param(name = {name:?}, regex = "value")]
struct Native(String);
impl FromStr for Native {{
    type Err = Infallible;
    fn from_str(value: &str) -> Result<Self, Self::Err> {{ Ok(Self(value.to_owned())) }}
}}
#[given({mode} = {expression:?})]
fn accept_value(world: &mut Registration, value: Native) {{
    assert_eq!(value.0, "value", "actual custom handler value");
    world.seen = true;
    println!("PHASE42_ACTUAL_HANDLER_VALUE=value");
}}
#[then("the actual handler executed")]
fn observed(world: &mut Registration) {{ assert!(world.seen); }}
#[tokio::main(flavor = "current_thread")]
async fn main() {{
    Registration::cucumber().with_default_cli().fail_on_skipped()
        .run_and_exit("registration.feature").await;
}}
"#, name = w.name);
    fs::write(workspace.consumer.join("tests/registration.rs"), &source).unwrap();
    fs::write(evidence.join("registration.rs"), &source).unwrap();
    let manifest = workspace.consumer.join("Cargo.toml");
    fs::copy(&manifest, evidence.join("Cargo.toml")).unwrap();
    let argv = ["test".to_owned(), "--offline".to_owned(), "--manifest-path".to_owned(), manifest.display().to_string(), "--test".to_owned(), "registration".to_owned()];
    let started = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(&argv).current_dir(&workspace.consumer).env("CARGO_TARGET_DIR", &workspace.target)
        .output().expect("named child BDD target must be available at the phase boundary");
    w.status = output.status.code();
    w.stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    w.stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    fs::write(evidence.join("stdout.log"), &output.stdout).unwrap();
    fs::write(evidence.join("stderr.log"), &output.stderr).unwrap();
    fs::write(evidence.join("receipt.json"), json!({"name": w.name, "mode": mode, "argv": argv,
        "cwd": workspace.consumer, "targetDir": workspace.target, "startedUnixMs": started,
        "finishedUnixMs": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(), "exitCode": w.status}).to_string()).unwrap();
    if workspace.consumer.join("Cargo.lock").exists() { fs::copy(workspace.consumer.join("Cargo.lock"), evidence.join("Cargo.lock")).unwrap(); }
    let mut oracle = Command::new("node").arg("tests/interop/expressions/parameter-names.mjs")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    oracle.stdin.take().unwrap().write_all(json!({"name": w.name}).to_string().as_bytes()).unwrap();
    let result = oracle.wait_with_output().unwrap();
    fs::write(evidence.join("oracle.stdout.json"), &result.stdout).unwrap();
    fs::write(evidence.join("oracle.stderr.log"), &result.stderr).unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    w.oracle = serde_json::from_slice(&result.stdout).expect("pinned registry oracle produces JSON");
}
#[when("an actual expr_js step definition is compiled at the completed BDD boundary")]
#[when("an actual expr_js step definition is compiled and executed at the completed BDD boundary")]
fn explicit(w: &mut Acceptance) { execute(w, "expr_js"); }
#[when("its existing derive and legacy expr step definition compile and execute at the completed BDD boundary")]
fn legacy(w: &mut Acceptance) { execute(w, "expr"); }
#[then("registration fails with a parameter-name diagnostic")]
fn rejected(w: &mut Acceptance) {
    assert_eq!(w.status, Some(101), "expected compiler rejection, evidence {}: {}", w.evidence.display(), w.stderr);
    assert!(w.stderr.contains(&format!("invalid JavaScript parameter name `{}`", w.name)), "{}", w.stderr);
    assert!(w.stderr.contains("tests/registration.rs"), "diagnostic must identify source: {}", w.stderr);
    assert!(!w.stdout.contains("PHASE42_ACTUAL_HANDLER_VALUE"));
}
#[then("the native handler receives the expected custom value")]
fn accepted(w: &mut Acceptance) {
    assert_eq!(w.status, Some(0), "child BDD failed, evidence {}: {}", w.evidence.display(), w.stderr);
    assert!(w.stdout.contains("PHASE42_ACTUAL_HANDLER_VALUE=value"), "actual handler must execute: {}", w.stdout);
}
#[then("the pinned JavaScript parameter registry rejects the same name")]
fn official_rejection(w: &mut Acceptance) { assert_eq!(w.oracle["accepted"], false); assert!(w.oracle["error"].as_str().unwrap().contains("Illegal character in parameter name")); }
#[then("the pinned JavaScript parameter registry accepts the same name")]
fn official_acceptance(w: &mut Acceptance) { assert_eq!(w.oracle["accepted"], true, "{}", w.oracle); }
#[tokio::main]
async fn main() {
    let temporary = tempfile::tempdir().unwrap();
    let consumer = temporary.path().join("consumer");
    fs::create_dir_all(consumer.join("tests")).unwrap();
    let repository = fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    let local_path = serde_json::to_string(&repository.display().to_string()).unwrap();
    fs::write(consumer.join("Cargo.toml"), format!(r#"[package]
name = "phase42-registration"
version = "0.0.0"
edition = "2024"
publish = false
[workspace]
[dependencies]
cucumber = {{ version = "=0.23.0", path = {local_path} }}
tokio = {{ version = "1.40", features = ["macros", "rt"] }}
[[test]]
name = "registration"
harness = false
"#)).unwrap();
    fs::write(consumer.join("registration.feature"), "Feature: Native registration\n Scenario: Invoke the registered handler\n  Given value value\n  Then the actual handler executed\n").unwrap();
    let evidence = repository.join("docs/compatibility/evidence/phase42-children")
        .join(format!("boundary-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()));
    fs::create_dir_all(&evidence).unwrap();
    WORKSPACE.set(Workspace { consumer, target: temporary.path().join("target"), evidence, sequence: AtomicUsize::new(0) }).ok().expect("one outer BDD workspace");
    Acceptance::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/expression_names").await;
}
