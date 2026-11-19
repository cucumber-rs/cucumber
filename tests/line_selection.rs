//! Phase34 original source location selection. Core acceptance needs no
//! Messages.
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use cucumber::{
    FilterOptions, Parser as _, StatsWriter as _, World as _, WriterExt as _,
    filter::LineSelectors,
    given,
    parser::{self, SourceFormat, SourceStore},
    step::{Collection, Context},
    then, when, writer,
};
use futures::{StreamExt as _, future::LocalBoxFuture};
use regex::Regex;
static EXECUTED: Mutex<Vec<String>> = Mutex::new(Vec::new());
#[derive(Debug, Default, cucumber::World)]
struct Selection {
    directory: Option<tempfile::TempDir>,
    path: PathBuf,
    source: String,
    locations: HashMap<String, usize>,
    format: SourceFormat,
    selectors: Option<LineSelectors>,
    store: SourceStore,
    executed: Vec<String>,
    compose: bool,
    cli_red: bool,
    internal_only: bool,
    errors: Vec<String>,
    generations_ok: bool,
    #[cfg(feature = "output-messages")]
    messages: Vec<serde_json::Value>,
}
#[derive(Debug, Default, cucumber::World)]
struct Execution;
fn execute(_: &mut Execution, context: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async move {
        EXECUTED
            .lock()
            .unwrap()
            .push(context.step.value.strip_prefix("execute ").unwrap().into());
    })
}
fn background(_: &mut Execution, _: Context) -> LocalBoxFuture<'_, ()> {
    Box::pin(async {})
}
fn collection() -> Collection<Execution> {
    Collection::new()
        .given(None, Regex::new("^execute .+$").unwrap(), execute)
        .given(None, Regex::new("^background$").unwrap(), background)
}
fn fixture_source(markdown: bool) -> (String, HashMap<String, usize>) {
    let lines = if markdown {
        vec![
            ("", "`@feature`"),
            ("feature", "# Feature: Source selection"),
            ("", "## Background: global"),
            ("", "* Given background"),
            ("", "`@healthy`"),
            ("scenario", "## Scenario: health"),
            ("", "* Given execute health"),
            ("rule", "## Rule: invoices"),
            ("background", "### Background: scoped"),
            ("", "* Given background"),
            ("", "`@scenario`"),
            ("outline", "### Scenario Outline: invoice <color>"),
            ("step", "* Given execute invoice <color>"),
            ("", "`@eligible`"),
            ("examples", "#### Examples: accepted"),
            ("table header", "  | color |"),
            ("separator", "  | --- |"),
            ("first row", "  | green |"),
            ("second row", "  | green |"),
            ("", "`@excluded`"),
            ("", "#### Examples: other"),
            ("", "  | color |"),
            ("", "  | --- |"),
            ("last row", "  | red |"),
        ]
    } else {
        vec![
            ("", "@feature"),
            ("feature", "Feature: Source selection"),
            ("", "  Background: global"),
            ("", "    Given background"),
            ("", "  @healthy"),
            ("scenario", "  Scenario: health"),
            ("", "    Given execute health"),
            ("rule", "  Rule: invoices"),
            ("background", "    Background: scoped"),
            ("", "      Given background"),
            ("", "    @scenario"),
            ("outline", "    Scenario Outline: invoice <color>"),
            ("step", "      Given execute invoice <color>"),
            ("", "      @eligible"),
            ("examples", "      Examples: accepted"),
            ("", "        description before table"),
            ("comment", "        # source gap before header"),
            ("table header", "        | color |"),
            ("", "        # source gap before first row"),
            ("first row", "        | green |"),
            ("", "        # source gap between duplicate rows"),
            ("second row", "        | green |"),
            ("", "      @excluded"),
            ("", "      Examples: other"),
            ("", "        | color |"),
            ("last row", "        | red |"),
        ]
    };
    let mut source = String::new();
    let mut locations = HashMap::new();
    for (index, (label, line)) in lines.iter().enumerate() {
        if !label.is_empty() {
            locations.insert((*label).into(), index + 1);
        }
        source.push_str(line);
        source.push('\n');
    }
    (source, locations)
}
#[given(expr = "a {string} selection source with comments duplicate rows and \
        Rule backgrounds")]
fn fixture(world: &mut Selection, format: String) {
    world.format = if format == "markdown" {
        SourceFormat::Markdown
    } else {
        assert_eq!(format, "plain");
        SourceFormat::Plain
    };
    let (source, locations) = fixture_source(format == "markdown");
    world.source = source;
    world.locations = locations;
    let directory = tempfile::tempdir().unwrap();
    world.path = directory.path().canonicalize().unwrap().join(
        if format == "markdown" {
            "selection.feature.md"
        } else {
            "selection.feature"
        },
    );
    std::fs::write(&world.path, &world.source).unwrap();
    world.directory = Some(directory);
}
#[given(expr = "its selected source locations are {string}")]
fn select_locations(world: &mut Selection, location: String) {
    let lines = if location == "first and last row" {
        vec![world.locations["first row"], world.locations["last row"]]
    } else {
        vec![world.locations[&location]]
    };
    world.selectors =
        Some(LineSelectors::new().lines(&world.path, lines).unwrap());
}
#[given("no external source store is configured")]
fn internal_only(world: &mut Selection) {
    world.internal_only = true;
}
#[given(
    "typed names tags and the caller predicate select only the eligible \
        green invoice"
)]
fn compose(world: &mut Selection) {
    world.compose = true;
}
#[given("the CLI selects red while the caller selects green")]
fn cli_red(world: &mut Selection) {
    world.cli_red = true;
}
#[given(
    "the first row is selected repeatedly by absolute relative and symlink \
        paths"
)]
fn aliases(world: &mut Selection) {
    let row = world.locations["first row"];
    let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
    let directory = tempfile::tempdir_in(&cwd).unwrap();
    world.path = directory.path().join("selection.feature");
    std::fs::write(&world.path, &world.source).unwrap();
    world.directory = Some(directory);
    let relative = world.path.strip_prefix(&cwd).unwrap().to_path_buf();
    let alias = world.directory.as_ref().unwrap().path().join("alias.feature");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&world.path, &alias).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&world.path, &alias).unwrap();
    world.selectors = Some(
        LineSelectors::new()
            .lines(&world.path, [row, row])
            .unwrap()
            .lines(relative, [row])
            .unwrap()
            .lines(alias, [row])
            .unwrap(),
    );
}
#[given("the source is selected by its first row and as a whole file")]
fn whole(world: &mut Selection) {
    world.selectors = Some(
        LineSelectors::new()
            .lines(&world.path, [world.locations["first row"]])
            .unwrap()
            .file(&world.path)
            .unwrap(),
    );
}
#[given("only a path outside parser discovery is selected")]
fn outside(world: &mut Selection) {
    world.selectors = Some(
        LineSelectors::new()
            .lines(world.path.with_file_name("absent.feature"), [1])
            .unwrap(),
    );
}
#[given("an empty explicit source selection")]
fn empty(world: &mut Selection) {
    world.selectors = Some(LineSelectors::new());
}
async fn run(world: &mut Selection, legacy: bool) {
    EXECUTED.lock().unwrap().clear();
    let mut output = Vec::new();
    let app = Execution::cucumber()
        .steps(collection())
        .max_concurrent_scenarios(1)
        .with_source_format(world.format);
    let app = if world.internal_only {
        app
    } else {
        app.with_source_store(world.store.clone())
    };
    let app = if legacy { app } else { app.with_gherkin43().unwrap() };
    let app = if world.compose {
        app.with_filter_options(
            FilterOptions::new()
                .name(Regex::new("^invoice (green|red)$").unwrap())
                .tag("@feature".parse().unwrap())
                .tag("@eligible".parse().unwrap()),
        )
    } else {
        app
    };
    let app = if let Some(selectors) = &world.selectors {
        app.with_line_selectors(selectors.clone())
    } else {
        app
    };
    let mut cli: cucumber::cli::Opts<_, _, _, cucumber::cli::Empty> =
        Default::default();
    if world.cli_red {
        cli.re_filter = Some(Regex::new("^invoice red$").unwrap());
    }
    let restrict = world.compose || world.cli_red;
    let stats = app
        .with_writer(
            writer::Basic::new(
                &mut output,
                writer::Coloring::Never,
                writer::Verbosity::Default,
            )
            .summarized(),
        )
        .with_cli(cli)
        .filter_run(&world.path, move |_, _, scenario| {
            !restrict || scenario.name == "invoice green"
        })
        .await;
    assert!(
        !stats.execution_has_failed(),
        "{}",
        String::from_utf8_lossy(&output)
    );
    world.executed = EXECUTED.lock().unwrap().clone();
}
#[when("the source executes with explicit line selection")]
async fn selected_run(world: &mut Selection) {
    run(world, false).await;
}
#[when("the source executes without line selectors")]
async fn legacy_run(world: &mut Selection) {
    run(world, true).await;
}
#[then(expr = "the selected scenario bodies are {string}")]
fn selected(world: &mut Selection, expected: String) {
    let mut expected = expected
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    expected.sort();
    world.executed.sort();
    assert_eq!(world.executed, expected);
}
#[then("origin metadata retains every original scenario and Examples row")]
fn origins(world: &mut Selection) {
    let snapshot = world.store.get(&world.path).unwrap();
    assert_eq!(snapshot.source(), world.source);
    let origins = snapshot.scenario_origins().unwrap();
    assert_eq!(origins.len(), 4);
    assert_eq!(
        (
            origins[0].rule_index(),
            origins[0].scenario_index(),
            origins[0].scenario_line(),
            origins[0].example_line()
        ),
        (None, 0, world.locations["scenario"], None)
    );
    for (index, label) in
        ["first row", "second row", "last row"].into_iter().enumerate()
    {
        assert_eq!(
            (
                origins[index + 1].rule_index(),
                origins[index + 1].scenario_index(),
                origins[index + 1].scenario_line(),
                origins[index + 1].example_line()
            ),
            (
                Some(0),
                index,
                world.locations["outline"],
                Some(world.locations[label])
            )
        );
    }
    let native = snapshot.feature();
    assert_ne!(
        native.rules[0].scenarios[0].examples[0].position.line + 2,
        world.locations["first row"],
        "fixture exposes synthetic-position error"
    );
}
#[given("an unconfigured source selector")]
fn unconfigured(_: &mut Selection) {}
#[when("a zero line and an empty path are requested")]
fn invalid(world: &mut Selection) {
    world.errors.push(
        LineSelectors::new()
            .lines("missing.feature", [0])
            .unwrap_err()
            .to_string(),
    );
    world.errors.push(LineSelectors::new().file("").unwrap_err().to_string());
}
#[then("both invalid values are rejected with meaningful errors")]
fn invalid_errors(world: &mut Selection) {
    assert_eq!(world.errors.len(), 2);
    assert!(world.errors[0].contains("positive"));
    assert!(world.errors[1].contains("empty"));
}
#[given("two discoverable sources and a selector for an absent third source")]
fn multiple(world: &mut Selection) {
    let directory = tempfile::tempdir().unwrap();
    world.path = directory.path().canonicalize().unwrap();
    for name in ["first", "second"] {
        std::fs::write(
            world.path.join(format!("{name}.feature")),
            format!(
                "Feature: {name}\n Scenario: {name}\n  Given execute {name}\n"
            ),
        )
        .unwrap();
    }
    world.selectors = Some(
        LineSelectors::new()
            .lines(world.path.join("absent.feature"), [2])
            .unwrap(),
    );
    world.directory = Some(directory);
}
#[when("each discovered source is selected by its scenario declaration")]
async fn multiple_run(world: &mut Selection) {
    let selectors = world
        .selectors
        .take()
        .unwrap()
        .lines(world.path.join("first.feature"), [2])
        .unwrap()
        .lines(world.path.join("second.feature"), [2])
        .unwrap();
    world.selectors = Some(selectors);
    run(world, false).await;
}
#[then(
    "both selected files execute once and the absent selector causes no error"
)]
fn multiple_selected(world: &mut Selection) {
    selected(world, "first, second".into());
}
async fn capture(
    world: &Selection,
) -> Vec<parser::Result<cucumber::gherkin::Feature>> {
    parser::Basic::new()
        .with_gherkin43()
        .unwrap()
        .with_source_store(world.store.clone())
        .parse(world.path.clone(), Default::default())
        .collect()
        .await
}
#[when(
    "a source snapshot is retained then its file is changed reparsed and \
        deleted"
)]
async fn generations(world: &mut Selection) {
    assert!(capture(world).await[0].is_ok());
    let first = world.store.get(&world.path).unwrap();
    let origins = first.scenario_origins().unwrap();
    std::fs::write(
        &world.path,
        "Feature: Changed\n Scenario: new\n  Given execute new\n",
    )
    .unwrap();
    assert!(capture(world).await[0].is_ok());
    let second = world.store.get(&world.path).unwrap();
    assert_eq!(second.scenario_origins().unwrap().len(), 1);
    std::fs::remove_file(&world.path).unwrap();
    assert!(capture(world).await[0].is_err());
    assert!(world.store.get(&world.path).is_none());
    assert_eq!(first.source(), world.source);
    assert_eq!(first.scenario_origins().unwrap(), origins);
    assert_eq!(second.scenario_origins().unwrap()[0].scenario_line(), 2);
    world.generations_ok = true;
}
#[then("retained origins still refer to their original source generation")]
fn retained(world: &mut Selection) {
    assert!(world.generations_ok);
}
#[cfg(feature = "output-messages")]
#[when("the source executes with source-backed Messages and line selection")]
async fn messages_run(world: &mut Selection) {
    EXECUTED.lock().unwrap().clear();
    let mut output = Vec::new();
    Execution::cucumber()
        .steps(collection())
        .max_concurrent_scenarios(1)
        .with_gherkin43()
        .unwrap()
        .with_source_store(world.store.clone())
        .with_line_selectors(world.selectors.clone().unwrap())
        .with_writer(writer::Messages::with_source_store::<Execution>(
            &mut output,
            world.store.clone(),
        ))
        .with_default_cli()
        .run(&world.path)
        .await;
    world.executed = EXECUTED.lock().unwrap().clone();
    world.messages = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
}
#[cfg(feature = "output-messages")]
#[then(
    "the report retains all Pickles but references only the selected \
        original row"
)]
fn messages_origin(world: &mut Selection) {
    let envelopes = |key: &str| {
        world
            .messages
            .iter()
            .filter_map(|message| message.get(key))
            .collect::<Vec<_>>()
    };
    let sources = envelopes("source");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["data"], world.source);
    let documents = envelopes("gherkinDocument");
    assert_eq!(documents.len(), 1);
    let rule = &documents[0]["feature"]["children"][2]["rule"];
    let scenario = &rule["children"][1]["scenario"];
    let row = &scenario["examples"][0]["tableBody"][1];
    assert_eq!(row["location"]["line"], world.locations["second row"]);
    let pickles = envelopes("pickle");
    assert_eq!(pickles.len(), 4);
    let cases = envelopes("testCase");
    assert_eq!(cases.len(), 1);
    let selected = pickles
        .iter()
        .find(|pickle| pickle["id"] == cases[0]["pickleId"])
        .unwrap();
    assert_eq!(selected["astNodeIds"][1], row["id"]);
    assert_eq!(envelopes("testCaseFinished").len(), 1);
    assert_eq!(envelopes("testRunFinished")[0]["success"], true);
}
#[tokio::main]
async fn main() {
    Selection::cucumber()
        .max_concurrent_scenarios(1)
        .with_filter_options(FilterOptions::new())
        .fail_on_skipped()
        .filter_run_and_exit(
            "tests/features/line_selection",
            |_, _, scenario| {
                cfg!(feature = "output-messages")
                    || !scenario
                        .tags
                        .iter()
                        .any(|tag| tag == "phase34_messages")
            },
        )
        .await;
}
