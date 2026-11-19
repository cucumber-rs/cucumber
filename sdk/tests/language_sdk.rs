//! Installed-language SDK behavior; invoked only at the complete phase
//! boundary.
use std::{
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use cucumber::{World as _, WriterExt as _, given, then, when};
static PASSED: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default, cucumber::World)]
struct InstalledSdk {
    case: String,
    output: Option<Output>,
}
#[given(expr = "the installed native SDK contract {string}")]
fn contract(world: &mut InstalledSdk, case: String) {
    world.case = case;
}
fn exercise(world: &mut InstalledSdk, control: bool) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let runtime_path = root.join("sdk/acceptance/runtime.json");
    let runtime: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&runtime_path)
            .expect("phase-end package preparation must precede acceptance"),
    )
    .unwrap();
    world.output = Some(
        Command::new(runtime["acceptancePython"].as_str().unwrap())
            .arg(root.join("sdk/acceptance/cases.py"))
            .arg(&runtime_path)
            .arg(&world.case)
            .arg(if control { "control" } else { "positive" })
            .current_dir(runtime["consumerRoot"].as_str().unwrap())
            .output()
            .expect("execute actual installed consumers"),
    );
}
#[when("the application exercises its real package boundary")]
fn application(world: &mut InstalledSdk) {
    exercise(world, false);
}
#[when("its declared negative control is exercised")]
fn control(world: &mut InstalledSdk) {
    exercise(world, true);
}
fn accepted(world: &InstalledSdk) {
    let output = world.output.as_ref().unwrap();
    assert!(
        output.status.success(),
        "{} failed: stdout={} stderr={}",
        world.case,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("explicit acceptance receipt");
    assert_eq!(receipt["case"], world.case);
    assert_eq!(receipt["status"], "PASS");
    assert!(receipt["observations"].as_array().is_some_and(|o| !o.is_empty()));
    PASSED.fetch_add(1, Ordering::SeqCst);
}
#[then("the result satisfies its independent source contract")]
fn result(world: &mut InstalledSdk) {
    accepted(world);
}
#[then("the false result is rejected without losing original evidence")]
fn rejected(world: &mut InstalledSdk) {
    accepted(world);
}
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let runtime_path = root.join("sdk/acceptance/runtime.json");
    let runtime: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&runtime_path).expect("prepared phase runtime"),
    )
    .unwrap();
    let args: Vec<_> = std::env::args().collect();
    let functional = args.iter().any(|arg| arg == "@sdk46");
    let controls = args.iter().any(|arg| arg == "@sdk46_negative");
    assert!(
        functional != controls,
        "select exactly one complete adopted SDK tag packet"
    );
    let messages_path = PathBuf::from(runtime["evidence"].as_str().unwrap())
        .join(if functional {
            "functional-messages.ndjson"
        } else {
            "controls-messages.ndjson"
        });
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&messages_path)
        .expect("outer BDD Messages evidence must never be overwritten");
    let sources = cucumber::parser::SourceStore::new();
    let writer =
        cucumber::writer::Basic::stdout::<InstalledSdk>().summarized().tee(
            cucumber::writer::Messages::with_source_store::<InstalledSdk>(
                output,
                sources.clone(),
            )
            .discard_stats_writes(),
        );
    futures::executor::block_on(
        InstalledSdk::cucumber()
            .max_concurrent_scenarios(1)
            .with_source_store(sources)
            .with_writer(writer)
            .fail_on_skipped()
            .run_and_exit(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/acceptance/features"
            )),
    );
    assert_eq!(
        PASSED.load(Ordering::SeqCst),
        11,
        "a phase packet must run exactly eleven adopted scenarios; \
            zero/partial/unfiltered selection is invalid"
    );
    if std::env::args().any(|arg| arg == "@sdk46_negative") {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let runtime_path = root.join("sdk/acceptance/runtime.json");
        let runtime: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&runtime_path).unwrap())
                .unwrap();
        let output =
            Command::new(runtime["acceptancePython"].as_str().unwrap())
                .arg(root.join("sdk/acceptance/finalize.py"))
                .arg(&runtime_path)
                .output()
                .expect("complete-packet cleanup");
        println!(
            "SDK cleanup stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        eprintln!(
            "SDK cleanup stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.status.success(),
            "phase acceptance cleanup failed: {}",
            output.status
        );
    }
}
