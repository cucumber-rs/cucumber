//! Phase33 explicit filter composition through real execution and captured source.
use std::{path::PathBuf, sync::Mutex};
use cucumber::{FilterOptions, World as _, cli, given, when, then, parser, runner, step::{Collection, Context}, writer};
use futures::future::LocalBoxFuture;
use regex::Regex;
use serde_json::Value;

const SOURCE: &str = "@feature\nFeature: Selection\n  @scenario\n  Scenario: health\n    Given execute health\n  @rule\n  Rule: Invoices\n    @scenario\n    Scenario Outline: invoice <color>\n      Given execute invoice <color>\n      @eligible\n      Examples:\n        | color |\n        | green |\n        | blue |\n      @excluded\n      Examples:\n        | color |\n        | red |\n";
static EXECUTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

#[derive(Debug, Default, cucumber::World)]
struct Selection {
    directory: Option<tempfile::TempDir>,
    path: PathBuf,
    options: Option<FilterOptions>,
    cli_mode: String,
    store: parser::SourceStore,
    messages: Vec<Value>,
    executed: Vec<String>,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution;
fn execute(_: &mut Execution, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        EXECUTED.lock().unwrap().push(context.step.value.strip_prefix("execute ").unwrap().to_owned());
    })
}
fn collection() -> Collection<Execution> {
    Collection::new().given(None, Regex::new("^execute .+$").unwrap(), execute)
}
fn configuration(name: &str) -> FilterOptions {
    let options = FilterOptions::new();
    match name {
        "names OR tags AND predicate" => options
            .name(Regex::new("^invoice green$").unwrap())
            .name(Regex::new("^invoice red$").unwrap())
            .tag("@feature and @rule and @scenario".parse().unwrap())
            .tag("@eligible".parse().unwrap()),
        "empty options" => options,
        "empty tag expression" => options.name(Regex::new("^invoice green$").unwrap()).tag("".parse().unwrap()),
        "missing repeated tag" => options.tag("@feature".parse().unwrap()).tag("@absent".parse().unwrap()),
        "expanded name" => options.name(Regex::new("^invoice green$").unwrap()),
        other => panic!("unknown configuration {other}"),
    }
}
#[given("the selection fixture with inherited tags and expanded names")]
fn fixture(world: &mut Selection) {
    let directory = tempfile::tempdir().unwrap();
    world.path = directory.path().canonicalize().unwrap().join("selection.feature");
    std::fs::write(&world.path, SOURCE).unwrap();
    world.directory = Some(directory);
    world.cli_mode = "none".into();
}
#[given(expr = "explicit filter configuration {string}")]
fn explicit(world: &mut Selection, name: String) { world.options = Some(configuration(&name)); }
#[given(expr = "CLI filter configuration {string}")]
fn cli_filter(world: &mut Selection, name: String) { world.cli_mode = name; }
#[given(expr = "legacy selection with CLI {string}")]
fn legacy(world: &mut Selection, name: String) { world.options = None; world.cli_mode = name; }

fn cli_options(mode: &str) -> cli::Opts<parser::basic::Cli, runner::basic::Cli, cli::Empty> {
    let (name, tags) = match mode {
        "none" => (None, None),
        "name alternative" => (Some("^health$"), None),
        "name and tag" => (Some("^invoice "), Some("@eligible")),
        "excluded name" => (Some("^invoice red$"), None),
        "excluded tag" => (None, Some("@excluded")),
        other => panic!("unknown CLI configuration {other}"),
    };
    // Deliberately spell every existing public field: no breaking CLI field added.
    // Both selectors can be provided programmatically; argv's legacy conflict remains.
    cli::Opts {
        re_filter: name.map(|name| Regex::new(name).unwrap()),
        tags_filter: tags.map(|tags| tags.parse().unwrap()),
        parser: Default::default(),
        runner: Default::default(),
        writer: cli::Empty,
        custom: cli::Empty,
    }
}
async fn run(world: &mut Selection, cloned: bool) {
    EXECUTED.lock().unwrap().clear();
    let mut output = Vec::new();
    let app = Execution::cucumber::<PathBuf>().with_writer(writer::Basic::new(
        Vec::<u8>::new(), writer::Coloring::Never, writer::Verbosity::Default,
    ));
    let app = match world.options.clone() {
        Some(options) => app.with_filter_options(options),
        None => app,
    };
    let app = if cloned { app.clone() } else { app };
    // Retention through type-changing builders is part of the public contract.
    app.with_parser(parser::Basic::new().with_gherkin43().unwrap().with_source_store(world.store.clone()))
        .with_runner(runner::Basic::<Execution>::default().steps(collection()))
        .max_concurrent_scenarios(1)
        .which_scenario(|_, _, _| cucumber::ScenarioType::Serial)
        .with_writer(writer::Messages::with_source_store::<Execution>(&mut output, world.store.clone()))
        .with_cli(cli_options(&world.cli_mode))
        .before(|_, _, _, _| Box::pin(async {}))
        .after(|_, _, _, _, _| Box::pin(async {}))
        .with_hooks(runner::Hooks::new())
        .with_run_hooks(runner::RunHooks::new())
        .filter_run(&world.path, |_, _, scenario| matches!(scenario.name.as_str(), "invoice green" | "health"))
        .await;
    world.executed = EXECUTED.lock().unwrap().clone();
    world.messages = String::from_utf8(output).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect();
}
#[when("the application selects scenarios with its caller predicate")]
async fn select(world: &mut Selection) { run(world, false).await; }
#[when("the executor is cloned and its parser runner writer CLI and hook adapters are replaced")]
async fn adapted(world: &mut Selection) { run(world, true).await; }
#[then(expr = "exactly {string} execute through real handlers")]
fn selected(world: &mut Selection, expected: String) {
    let mut expected = expected.split(',').map(str::trim).filter(|name| !name.is_empty()).map(str::to_owned).collect::<Vec<_>>();
    expected.sort(); world.executed.sort();
    assert_eq!(world.executed, expected, "actual handler invocations");
    let pickles = world.messages.iter().filter_map(|message| message.get("pickle")).collect::<Vec<_>>();
    let mut reported = world.messages.iter().filter_map(|message| message.get("testCase")).map(|case| {
        pickles.iter().find(|pickle| pickle["id"] == case["pickleId"]).expect("case must reference a source Pickle")["name"].as_str().unwrap().to_owned()
    }).collect::<Vec<_>>();
    reported.sort(); assert_eq!(reported, expected, "reported cases agree with execution");
    assert_eq!(world.messages.iter().filter(|message| message.get("testCaseFinished").is_some()).count(), expected.len());
    for result in world.messages.iter().filter_map(|message| message.get("testStepFinished")) {
        assert_eq!(result["testStepResult"]["status"], "PASSED");
    }
    let finished = world.messages.iter().filter_map(|message| message.get("testRunFinished")).collect::<Vec<_>>();
    assert_eq!(finished.len(), 1); assert_eq!(finished[0]["success"], true);
}
#[then("the captured source retains every original scenario and example row")]
fn captured(world: &mut Selection) {
    let snapshot = world.store.get(&world.path).expect("raw snapshot exists even if no scenarios selected");
    assert_eq!(snapshot.source(), SOURCE);
    let graph = writer::messages::source::SourceGraph::from_snapshot(world.path.to_str().unwrap(), &snapshot).unwrap();
    let names = graph.pickles().iter().map(|pickle| pickle["name"].as_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(names, ["health", "invoice green", "invoice blue", "invoice red"]);
    let feature = &graph.gherkin_document()["feature"];
    assert_eq!(feature["children"][1]["rule"]["children"][0]["scenario"]["name"], "invoice <color>");
    let examples = &feature["children"][1]["rule"]["children"][0]["scenario"]["examples"];
    assert_eq!(examples[0]["tableBody"].as_array().unwrap().len(), 2);
    assert_eq!(examples[1]["tableBody"].as_array().unwrap().len(), 1);
    if let Some(source) = world.messages.iter().find_map(|message| message.get("source")) {
        assert_eq!(source["data"], SOURCE);
    }
}
#[tokio::main]
async fn main() {
    Selection::cucumber().max_concurrent_scenarios(1).fail_on_skipped().run_and_exit("tests/features/composable_filters").await;
}
