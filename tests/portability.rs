//! Phase 2 acceptance through a separately compiled, relocated application.
use std::{fs, path::Path, process::Command};

use cucumber::{World as _, given, then, when};

#[derive(Debug, Default, cucumber::World)]
struct Portability {
    workspace: Option<tempfile::TempDir>,
    output: String,
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[given("a relocated Cucumber source checkout with its vendored parser")]
fn relocate(world: &mut Portability) {
    let workspace = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    let relocated = workspace.path().join("foundation");
    fs::create_dir(&relocated).unwrap();
    for directory in ["src", "vendor", "tests"] {
        copy_tree(&source.join(directory), &relocated.join(directory));
    }
    fs::copy(source.join("Cargo.toml"), relocated.join("Cargo.toml")).unwrap();
    // Include the optional default macro crate as ordinary source, without any
    // dependency on the original checkout or its Cargo configuration.
    copy_tree(&source.join("codegen/src"), &relocated.join("codegen/src"));
    copy_tree(&source.join("codegen/tests"), &relocated.join("codegen/tests"));
    fs::copy(
        source.join("codegen/Cargo.toml"),
        relocated.join("codegen/Cargo.toml"),
    )
    .unwrap();
    world.workspace = Some(workspace);
}

#[given("an application depending only on that relocated Cucumber checkout")]
fn application(world: &mut Portability) {
    let root = world.workspace.as_ref().unwrap().path();
    let app = root.join("application");
    fs::create_dir_all(app.join("src")).unwrap();
    fs::write(
        app.join("Cargo.toml"),
        r#"[package]
name = "portable-cucumber-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
cucumber = { path = "../foundation" }

[workspace]
"#,
    )
    .unwrap();
    fs::write(
        app.join("src/main.rs"),
        "\
            fn main() {\n\
            \x20   let source = r#\"Feature: Imported \
            transactions\n\
            \x20 Scenario: Preserve imported values\n\
            \x20   Given a payment\n\
            \x20     | amount | 42 |\n\
            \x20     \"\"\"\n\
            \x20     invoice 42\n\
            \x20     \"\"\"\n\
            \x20   Then a separate step\n\
            \"#;\n\
            \x20   let feature = \
            cucumber::gherkin::Feature::parse(source, \
            Default::default())\n\
            \x20       .expect(\"table-before-docstring must \
            parse through cucumber's public API\");\n\
            \x20   let steps = &feature.scenarios[0].steps;\n\
            \x20   assert_eq!(steps.len(), 2);\n\
            \x20   assert_eq!(steps[0].table.as_ref().unwrap().ro\
            ws, vec![vec![\"amount\", \"42\"]]);\n\
            \x20   assert_eq!(steps[0].docstring.as_deref(), \
            Some(\"\\ninvoice 42\\n\"));\n\
            \x20   assert_eq!(steps[1].value, \"a separate \
            step\");\n\
            \x20   assert!(steps[1].table.is_none());\n\
            \x20   assert!(steps[1].docstring.is_none());\n\
            \x20   println!(\"portable consumer observed exact \
            table, doc string and following step\");\n\
            }\n",
    )
    .unwrap();
}

#[when("the application parses a step with a table followed by a doc string")]
fn run_application(world: &mut Portability) {
    let app = world.workspace.as_ref().unwrap().path().join("application");
    // This compilation and execution is the action of the external-consumer
    // scenario, not a separate preflight check or generic Rust test suite.
    // A distinct cache prevents a nested Cargo target-directory lock.
    let output = Command::new(env!("CARGO"))
        .args(["run", "--offline", "--quiet"])
        .current_dir(app)
        .env(
            "CARGO_TARGET_DIR",
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/portability-consumer"),
        )
        .output()
        .expect("launch the relocated application");
    assert!(
        output.status.success(),
        "external consumer failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    world.output = String::from_utf8(output.stdout).unwrap();
}

#[then("the application observes both step arguments unchanged")]
fn observed(world: &mut Portability) {
    assert_eq!(
        world.output.trim(),
        "portable consumer observed exact table, doc string and following \
            step"
    );
}

#[then("the application uses no sibling parser checkout or Cargo patch")]
fn isolated(world: &mut Portability) {
    let root = world.workspace.as_ref().unwrap().path();
    assert!(!root.join("gherkin").exists());
    assert!(!root.join(".cargo").exists());
    assert!(!root.join("foundation/.cargo").exists());
    assert!(!root.join("application/.cargo").exists());
    for manifest in ["application/Cargo.toml", "foundation/Cargo.toml"] {
        let text = fs::read_to_string(root.join(manifest)).unwrap();
        assert!(!text.contains("[patch."));
        assert!(!text.contains("../gherkin"));
    }
    let lock = fs::read_to_string(root.join("application/Cargo.lock")).unwrap();
    let parser = lock
        .split("[[package]]")
        .find(|entry| entry.contains("name = \"gherkin\""))
        .unwrap();
    assert!(
        !parser.contains("source ="),
        "parser must resolve as a relative path dependency"
    );
    assert!(parser.contains("version = \"0.16.0\""));
}

#[tokio::main]
async fn main() {
    Portability::cucumber()
        .fail_on_skipped()
        .run_and_exit("tests/features/portability")
        .await;
}
