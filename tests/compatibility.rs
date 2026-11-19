//! Phase 1 acceptance: exercise the public parser/runner pipeline with
//! features.
use std::sync::{Arc, Mutex};

use cucumber::{
    StatsWriter as _, World as _, WriterExt as _,
    gherkin::Step as GherkinStep,
    given,
    step::{Collection, Context, Matching},
    then, when,
    writer::{self, Coloring},
};
use regex::Regex;

#[derive(Debug, Default, cucumber::World)]
struct Compatibility {
    matching: Matching,
    definitions: Collection<Account>,
    feature: String,
    passed: usize,
    failed: usize,
    skipped: usize,
    parsing_errors: usize,
    output: String,
    observed: Account,
}

#[derive(Clone, Debug, Default, cucumber::World)]
struct Account {
    balances: Vec<String>,
    payments: Vec<(String, String)>,
}

fn balance<'a>(
    world: &'a mut Account,
    ctx: Context,
) -> futures::future::LocalBoxFuture<'a, ()> {
    Box::pin(async move {
        assert_eq!(ctx.matches[1].0.as_deref(), Some("balance"));
        world.balances.push(ctx.matches[1].1.clone());
    })
}

fn payment<'a>(
    world: &'a mut Account,
    ctx: Context,
) -> futures::future::LocalBoxFuture<'a, ()> {
    Box::pin(async move {
        let doc = ctx.step.docstring.expect("document reaches step body");
        let table = ctx.step.table.expect("table reaches step body");
        assert_eq!(table.rows[0][0], "amount");
        world.payments.push((doc, table.rows[0][1].clone()));
    })
}

#[given("keyword independent matching")]
fn independent(world: &mut Compatibility) {
    world.matching = Matching::KeywordIndependent;
}

#[given("default matching")]
fn legacy(world: &mut Compatibility) {
    world.matching = Matching::default();
}

#[given(regex = r"^a (Given|When|Then) definition for an account balance$")]
fn definition(world: &mut Compatibility, keyword: String) {
    let regex =
        Regex::new(r"^the account balance is (?P<balance>\d+)$").unwrap();
    let definitions = std::mem::take(&mut world.definitions);
    world.definitions = match keyword.as_str() {
        "Given" => definitions.given(None, regex, balance),
        "When" => definitions.when(None, regex, balance),
        "Then" => definitions.then(None, regex, balance),
        _ => unreachable!(),
    };
}

#[given("a feature containing:")]
fn feature(world: &mut Compatibility, step: &GherkinStep) {
    world.feature = step.docstring.clone().expect("feature document");
}

#[when("the feature is executed")]
async fn execute(world: &mut Compatibility) {
    let dir = tempfile::tempdir().unwrap();
    let feature_path = dir.path().join("acceptance.feature");
    std::fs::write(&feature_path, &world.feature).unwrap();
    let observations = Arc::new(Mutex::new(Account::default()));
    let collected = Arc::clone(&observations);
    let definitions = world.definitions.clone().given(
        None,
        Regex::new("^the payment is submitted$").unwrap(),
        payment,
    );
    let mut output = Vec::new();
    let report = Account::cucumber()
        .steps(definitions)
        .step_matching(world.matching)
        .max_concurrent_scenarios(1)
        .after(move |_, _, _, _, account| {
            let collected = Arc::clone(&collected);
            Box::pin(async move {
                if let Some(account) = account {
                    let mut collected = collected.lock().unwrap();
                    collected.balances.extend(account.balances.clone());
                    collected.payments.extend(account.payments.clone());
                }
            })
        })
        .with_writer(
            writer::Basic::new(&mut output, Coloring::Never, 0).summarized(),
        )
        .with_default_cli()
        .run(feature_path)
        .await;
    world.passed = report.passed_steps();
    world.failed = report.failed_steps();
    world.skipped = report.skipped_steps();
    world.parsing_errors = report.parsing_errors();
    assert_eq!(report.hook_errors(), 0, "observation hook must succeed");
    drop(report);
    world.output = String::from_utf8(output).unwrap();
    world.observed = observations.lock().unwrap().clone();
}

#[then(
    regex = r"^(\d+) steps pass, (\d+) steps fail and (\d+) steps are skipped$"
)]
fn outcomes(
    world: &mut Compatibility,
    passed: usize,
    failed: usize,
    skipped: usize,
) {
    assert_eq!(world.parsing_errors, 0, "{}", world.output);
    assert_eq!(
        (world.passed, world.failed, world.skipped),
        (passed, failed, skipped),
        "{}",
        world.output
    );
}

#[then(regex = r#"^the observed balances are "([^"]*)"$"#)]
fn balances(world: &mut Compatibility, expected: String) {
    assert_eq!(world.observed.balances.join(","), expected);
}

#[then("the output reports an ambiguous match")]
fn ambiguous(world: &mut Compatibility) {
    assert!(
        world.output.to_lowercase().contains("ambiguous"),
        "{}",
        world.output
    );
}

#[then(regex = r#"^the observed payment is "([^"]*)" with amount "([^"]*)"$"#)]
fn observed_payment(
    world: &mut Compatibility,
    document: String,
    amount: String,
) {
    // Preserve the native parser's existing delimiter-adjacent newlines.
    // Argument ordering must not silently change existing consumers' values.
    assert_eq!(
        world.observed.payments,
        vec![(format!("\n{document}\n"), amount)]
    );
}

#[then("the feature is rejected before steps execute")]
fn rejected(world: &mut Compatibility) {
    assert_eq!(world.parsing_errors, 1, "{}", world.output);
    assert_eq!((world.passed, world.failed, world.skipped), (0, 0, 0));
    assert!(world.observed.balances.is_empty());
    assert!(world.observed.payments.is_empty());
}

#[tokio::main]
async fn main() {
    Compatibility::cucumber()
        .fail_on_skipped()
        .run_and_exit("tests/features/compatibility")
        .await;
}
