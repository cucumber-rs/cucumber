//! Scenario storage and management for the Basic runner.

use std::{
    cmp,
    collections::HashMap,
    iter, mem,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use futures::{
    channel::mpsc,
    lock::Mutex,
    TryStreamExt as _,
};
use itertools::Itertools as _;

use crate::{
    event::{self, source::Source},
    feature::Ext as _,
};

use super::{
    cli_and_types::{Cli, RetryOptions, RetryOptionsFn, RetryOptionsWithDeadline, ScenarioType},
    supporting_structures::{IsFailed, IsRetried, ScenarioId},
};

/// [`Scenario`]s storage.
///
/// [`Scenario`]: gherkin::Scenario
type Scenarios = HashMap<
    ScenarioType,
    Vec<(
        ScenarioId,
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
        Option<RetryOptionsWithDeadline>,
    )>,
>;

/// Alias of a [`Features::insert_scenarios()`] argument.
type InsertedScenarios = HashMap<
    ScenarioType,
    Vec<(
        ScenarioId,
        Source<gherkin::Feature>,
        Option<Source<gherkin::Rule>>,
        Source<gherkin::Scenario>,
        Option<RetryOptions>,
    )>,
>;

/// Storage sorted by [`ScenarioType`] [`Feature`]'s [`Scenario`]s.
///
/// [`Feature`]: gherkin::Feature
/// [`Scenario`]: gherkin::Scenario
#[derive(Clone, Default)]
pub struct Features {
    /// Storage itself.
    scenarios: Arc<Mutex<Scenarios>>,

    /// Indicates whether all parsed [`Feature`]s are sorted and stored.
    ///
    /// [`Feature`]: gherkin::Feature
    finished: Arc<AtomicBool>,
}

impl Features {
    /// Splits [`Feature`] into [`Scenario`]s, sorts by [`ScenarioType`] and
    /// stores them.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    pub async fn insert<Which>(
        &self,
        feature: gherkin::Feature,
        which_scenario: &Which,
        retry: &RetryOptionsFn,
        cli: &Cli,
    ) where
        Which: Fn(
                &gherkin::Feature,
                Option<&gherkin::Rule>,
                &gherkin::Scenario,
            ) -> ScenarioType
            + 'static,
    {
        let feature = Source::new(feature);

        let local = feature
            .scenarios
            .iter()
            .map(|s| (None, s))
            .chain(feature.rules.iter().flat_map(|r| {
                let rule = Some(Source::new(r.clone()));
                r.scenarios
                    .iter()
                    .map(|s| (rule.clone(), s))
                    .collect::<Vec<_>>()
            }))
            .map(|(rule, scenario)| {
                let retries = retry(&feature, rule.as_deref(), scenario, cli);
                (
                    ScenarioId::new(),
                    feature.clone(),
                    rule,
                    Source::new(scenario.clone()),
                    retries,
                )
            })
            .into_group_map_by(|(_, f, r, s, _)| {
                which_scenario(f, r.as_ref().map(AsRef::as_ref), s)
            });

        self.insert_scenarios(local).await;
    }

    /// Inserts the provided retried [`Scenario`] into this [`Features`]
    /// storage.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub async fn insert_retried_scenario(
        &self,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        scenario_ty: ScenarioType,
        retries: Option<RetryOptions>,
    ) {
        self.insert_scenarios(
            iter::once((
                scenario_ty,
                vec![(ScenarioId::new(), feature, rule, scenario, retries)],
            ))
            .collect(),
        )
        .await;
    }

    /// Inserts the provided [`Scenario`]s into this [`Features`] storage.
    ///
    /// [`Scenario`]: gherkin::Scenario
    async fn insert_scenarios(&self, scenarios: InsertedScenarios) {
        let now = Instant::now();

        let mut with_retries = HashMap::<_, Vec<_>>::new();
        let mut without_retries: Scenarios = HashMap::new();
        #[expect(clippy::iter_over_hash_type, reason = "order doesn't matter")]
        for (which, values) in scenarios {
            for (id, f, r, s, ret) in values {
                match ret {
                    ret @ (None
                    | Some(RetryOptions {
                        retries: crate::event::Retries { current: 0, .. },
                        ..
                    })) => {
                        // `Retries::current` is `0`, so this `Scenario` run is
                        // initial, and we don't need to wait for retry delay.
                        let ret = ret.map(RetryOptions::without_deadline);
                        without_retries
                            .entry(which)
                            .or_default()
                            .push((id, f, r, s, ret));
                    }
                    Some(ret) => {
                        let ret = ret.with_deadline(now);
                        with_retries
                            .entry(which)
                            .or_default()
                            .push((id, f, r, s, ret));
                    }
                }
            }
        }

        let mut storage = self.scenarios.lock().await;

        #[expect(clippy::iter_over_hash_type, reason = "order doesn't matter")]
        for (which, values) in with_retries {
            let ty_storage = storage.entry(which).or_default();
            for (id, f, r, s, ret) in values {
                ty_storage.insert(0, (id, f, r, s, Some(ret)));
            }
        }

        if without_retries.contains_key(&ScenarioType::Serial) {
            // If there are Serial Scenarios we insert all Serial and Concurrent
            // Scenarios in front.
            // This is done to execute them closely to one another, so the
            // output wouldn't hang on executing other Concurrent Scenarios.
            #[expect(
                clippy::iter_over_hash_type,
                reason = "order doesn't matter"
            )]
            for (which, mut values) in without_retries {
                let old = mem::take(storage.entry(which).or_default());
                values.extend(old);
                storage.entry(which).or_default().extend(values);
            }
        } else {
            // If there are no Serial Scenarios, we just extend already existing
            // Concurrent Scenarios.
            #[expect(
                clippy::iter_over_hash_type,
                reason = "order doesn't matter"
            )]
            for (which, values) in without_retries {
                storage.entry(which).or_default().extend(values);
            }
        }
    }

    /// Returns [`Scenario`]s which are ready to run and the minimal deadline of
    /// all retried [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub async fn get(
        &self,
        max_concurrent_scenarios: Option<usize>,
    ) -> (
        Vec<(
            ScenarioId,
            Source<gherkin::Feature>,
            Option<Source<gherkin::Rule>>,
            Source<gherkin::Scenario>,
            ScenarioType,
            Option<RetryOptions>,
        )>,
        Option<Duration>,
    ) {
        use RetryOptionsWithDeadline as WithDeadline;
        use ScenarioType::{Concurrent, Serial};

        if max_concurrent_scenarios == Some(0) {
            return (Vec::new(), None);
        }

        let mut min_dur = None;
        let mut drain =
            |storage: &mut Vec<(_, _, _, _, Option<WithDeadline>)>,
             ty,
             count: Option<usize>| {
                let mut i = 0;
                let drained = storage
                    .extract_if(.., |(_, _, _, _, ret)| {
                        // Because of retries involved, we cannot just specify
                        // `..count` range to `.extract_if()`.
                        if count.filter(|c| i >= *c).is_some() {
                            return false;
                        }

                        ret.as_ref()
                            .and_then(WithDeadline::left_until_retry)
                            .map_or_else(
                                || {
                                    i += 1;
                                    true
                                },
                                |left| {
                                    min_dur = min_dur
                                        .map(|min| cmp::min(min, left))
                                        .or(Some(left));
                                    false
                                },
                            )
                    })
                    .map(|(id, f, r, s, ret)| {
                        (id, f, r, s, ty, ret.map(Into::into))
                    })
                    .collect::<Vec<_>>();
                (!drained.is_empty()).then_some(drained)
            };

        let mut guard = self.scenarios.lock().await;
        let scenarios = guard
            .get_mut(&Serial)
            .and_then(|storage| drain(storage, Serial, Some(1)))
            .or_else(|| {
                guard.get_mut(&Concurrent).and_then(|storage| {
                    drain(storage, Concurrent, max_concurrent_scenarios)
                })
            })
            .unwrap_or_default();

        (scenarios, min_dur)
    }

    /// Marks that there will be no more [`Feature`]s to execute.
    ///
    /// [`Feature`]: gherkin::Feature
    pub fn finish(&self) {
        self.finished.store(true, Ordering::SeqCst);
    }

    /// Indicates whether there are more [`Feature`]s to execute.
    ///
    /// `fail_fast` argument indicates whether not yet executed scenarios should
    /// be omitted.
    ///
    /// [`Feature`]: gherkin::Feature
    pub async fn is_finished(&self, fail_fast: bool) -> bool {
        self.finished.load(Ordering::SeqCst)
            && (fail_fast
                || self.scenarios.lock().await.values().all(Vec::is_empty))
    }
}

/// Alias of a [`mpsc::UnboundedSender`] that notifies about finished
/// [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
pub type FinishedFeaturesSender = mpsc::UnboundedSender<(
    ScenarioId,
    Source<gherkin::Feature>,
    Option<Source<gherkin::Rule>>,
    IsFailed,
    IsRetried,
)>;

/// Alias of a [`mpsc::UnboundedReceiver`] that receives events about finished
/// [`Feature`]s.
///
/// [`Feature`]: gherkin::Feature
pub type FinishedFeaturesReceiver = mpsc::UnboundedReceiver<(
    ScenarioId,
    Source<gherkin::Feature>,
    Option<Source<gherkin::Rule>>,
    IsFailed,
    IsRetried,
)>;

/// Stores currently running [`Rule`]s and [`Feature`]s and notifies about their
/// state of completion.
///
/// [`Feature`]: gherkin::Feature
/// [`Rule`]: gherkin::Rule
pub struct FinishedRulesAndFeatures {
    /// Number of finished [`Scenario`]s of [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Scenario`]: gherkin::Scenario
    features_scenarios_count: HashMap<Source<gherkin::Feature>, usize>,

    /// Number of finished [`Scenario`]s of [`Rule`].
    ///
    /// We also store path to a [`Feature`], so [`Rule`]s with same names and
    /// spans in different `.feature` files will have different hashes.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    /// [`Scenario`]: gherkin::Scenario
    rule_scenarios_count:
        HashMap<(Source<gherkin::Feature>, Source<gherkin::Rule>), usize>,

    /// Receiver for notifying state of [`Scenario`]s completion.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub finished_receiver: FinishedFeaturesReceiver,
}

impl FinishedRulesAndFeatures {
    /// Creates a new [`FinishedRulesAndFeatures`] store.
    pub fn new(finished_receiver: FinishedFeaturesReceiver) -> Self {
        Self {
            features_scenarios_count: HashMap::new(),
            rule_scenarios_count: HashMap::new(),
            finished_receiver,
        }
    }

    /// Marks [`Rule`]'s [`Scenario`] as finished and returns [`Rule::Finished`]
    /// event if no [`Scenario`]s left.
    ///
    /// [`Rule`]: gherkin::Rule
    /// [`Rule::Finished`]: event::Rule::Finished
    /// [`Scenario`]: gherkin::Scenario
    pub fn rule_scenario_finished<W>(
        &mut self,
        feature: Source<gherkin::Feature>,
        rule: Source<gherkin::Rule>,
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if is_retried {
            return None;
        }

        let finished_scenarios = self
            .rule_scenarios_count
            .get_mut(&(feature.clone(), rule.clone()))
            .unwrap_or_else(|| panic!("no `Rule: {}`", rule.name));
        *finished_scenarios += 1;
        (rule.scenarios.len() == *finished_scenarios).then(|| {
            _ = self
                .rule_scenarios_count
                .remove(&(feature.clone(), rule.clone()));
            event::Cucumber::rule_finished(feature, rule)
        })
    }

    /// Marks [`Feature`]'s [`Scenario`] as finished and returns
    /// [`Feature::Finished`] event if no [`Scenario`]s left.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Feature::Finished`]: event::Feature::Finished
    /// [`Scenario`]: gherkin::Scenario
    pub fn feature_scenario_finished<W>(
        &mut self,
        feature: Source<gherkin::Feature>,
        is_retried: bool,
    ) -> Option<event::Cucumber<W>> {
        if is_retried {
            return None;
        }

        let finished_scenarios = self
            .features_scenarios_count
            .get_mut(&feature)
            .unwrap_or_else(|| panic!("no `Feature: {}`", feature.name));
        *finished_scenarios += 1;
        let scenarios = feature.count_scenarios();
        (scenarios == *finished_scenarios).then(|| {
            _ = self.features_scenarios_count.remove(&feature);
            event::Cucumber::feature_finished(feature)
        })
    }

    /// Marks all the unfinished [`Rule`]s and [`Feature`]s as finished, and
    /// returns all the appropriate finished events.
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Rule`]: gherkin::Rule
    pub fn finish_all_rules_and_features<W>(
        &mut self,
    ) -> impl Iterator<Item = event::Cucumber<W>> {
        self.rule_scenarios_count
            .drain()
            .map(|((feat, rule), _)| event::Cucumber::rule_finished(feat, rule))
            .chain(
                self.features_scenarios_count
                    .drain()
                    .map(|(feat, _)| event::Cucumber::feature_finished(feat)),
            )
    }

    /// Marks [`Scenario`]s as started and returns [`Rule::Started`] and
    /// [`Feature::Started`] if given [`Scenario`] was first for particular
    /// [`Rule`] or [`Feature`].
    ///
    /// [`Feature`]: gherkin::Feature
    /// [`Feature::Started`]: event::Feature::Started
    /// [`Rule`]: gherkin::Rule
    /// [`Rule::Started`]: event::Rule::Started
    /// [`Scenario`]: gherkin::Scenario
    pub fn start_scenarios<W, R>(
        &mut self,
        runnable: R,
    ) -> impl Iterator<Item = event::Cucumber<W>> + use<W, R>
    where
        R: AsRef<
            [(
                ScenarioId,
                Source<gherkin::Feature>,
                Option<Source<gherkin::Rule>>,
                Source<gherkin::Scenario>,
                ScenarioType,
                Option<RetryOptions>,
            )],
        >,
    {
        let runnable = runnable.as_ref();

        let mut started_features = Vec::new();
        for feature in runnable.iter().map(|(_, f, ..)| f.clone()).dedup() {
            _ = self
                .features_scenarios_count
                .entry(feature.clone())
                .or_insert_with(|| {
                    started_features.push(feature);
                    0
                });
        }

        let mut started_rules = Vec::new();
        for (feat, rule) in runnable
            .iter()
            .filter_map(|(_, feat, rule, _, _, _)| {
                rule.clone().map(|r| (feat.clone(), r))
            })
            .dedup()
        {
            _ = self
                .rule_scenarios_count
                .entry((feat.clone(), rule.clone()))
                .or_insert_with(|| {
                    started_rules.push((feat, rule));
                    0
                });
        }

        started_features
            .into_iter()
            .map(event::Cucumber::feature_started)
            .chain(
                started_rules
                    .into_iter()
                    .map(|(f, r)| event::Cucumber::rule_started(f, r)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc;
    use std::sync::Arc;
    use crate::test_utils::common::TestWorld;
    use crate::runner::basic::{RetryOptions, Cli, RetryOptionsFn};

    #[tokio::test]
    async fn test_features_empty() {
        let features = Features::default();
        features.finish();
        
        assert!(features.is_finished(false).await);
        assert!(features.is_finished(true).await);
    }

    #[tokio::test]
    async fn test_features_get_empty() {
        let features = Features::default();
        let (scenarios, min_dur) = features.get(Some(5)).await;
        
        assert!(scenarios.is_empty());
        assert!(min_dur.is_none());
    }

    #[tokio::test]
    async fn test_features_get_zero_concurrency() {
        let features = Features::default();
        let (scenarios, min_dur) = features.get(Some(0)).await;
        
        assert!(scenarios.is_empty());
        assert!(min_dur.is_none());
    }

    #[tokio::test]
    async fn test_features_insert_retried_scenario() {
        let features = Features::default();
        
        let feature = Source::new(gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
        });
        
        let scenario = Source::new(gherkin::Scenario {
            tags: vec![],
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            description: None,
            steps: vec![],
            examples: vec![],
        });
        
        features.insert_retried_scenario(
            feature,
            None,
            scenario,
            ScenarioType::Concurrent,
            None,
        ).await;
        
        let (scenarios, _) = features.get(Some(5)).await;
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].4, ScenarioType::Concurrent);
    }

    #[tokio::test]
    async fn test_features_insert_with_which_scenario() {
        let features = Features::default();
        
        let feature = gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            description: None,
            background: None,
            scenarios: vec![
                gherkin::Scenario {
                    tags: vec!["@serial".to_string()],
                    keyword: "Scenario".to_string(),
                    name: "Serial Scenario".to_string(),
                    span: gherkin::Span { start: 0, end: 0 },
                    position: gherkin::LineCol { line: 1, col: 1 },
                    description: None,
                    steps: vec![],
                    examples: vec![],
                },
                gherkin::Scenario {
                    tags: vec![],
                    keyword: "Scenario".to_string(),
                    name: "Concurrent Scenario".to_string(),
                    span: gherkin::Span { start: 0, end: 0 },
                    position: gherkin::LineCol { line: 1, col: 1 },
                    description: None,
                    steps: vec![],
                    examples: vec![],
                },
            ],
            rules: vec![],
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };
        
        let which_scenario = |_: &gherkin::Feature, _: Option<&gherkin::Rule>, scenario: &gherkin::Scenario| {
            if scenario.tags.contains(&"@serial".to_string()) {
                ScenarioType::Serial
            } else {
                ScenarioType::Concurrent
            }
        };
        
        let retry_fn: RetryOptionsFn = Arc::new(|_: &gherkin::Feature, _: Option<&gherkin::Rule>, _: &gherkin::Scenario, _: &Cli| -> Option<RetryOptions> { None });
        let cli = Cli::default();
        
        features.insert(feature, &which_scenario, &retry_fn, &cli).await;
        
        // Should get serial scenario first
        let (scenarios, _) = features.get(Some(5)).await;
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].4, ScenarioType::Serial);
        
        // Then concurrent scenario
        let (scenarios, _) = features.get(Some(5)).await;
        assert_eq!(scenarios.len(), 1);
        assert_eq!(scenarios[0].4, ScenarioType::Concurrent);
    }

    #[test]
    fn test_finished_rules_and_features_new() {
        let (_, receiver) = mpsc::unbounded();
        let storage = FinishedRulesAndFeatures::new(receiver);
        
        assert!(storage.features_scenarios_count.is_empty());
        assert!(storage.rule_scenarios_count.is_empty());
    }

    #[test]
    fn test_finished_rules_and_features_start_scenarios() {
        let (_, receiver) = mpsc::unbounded();
        let mut storage = FinishedRulesAndFeatures::new(receiver);
        
        let feature = Source::new(gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
        });
        
        let scenario = Source::new(gherkin::Scenario {
            tags: vec![],
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            description: None,
            steps: vec![],
            examples: vec![],
        });
        
        let runnable = vec![(
            ScenarioId::new(),
            feature,
            None,
            scenario,
            ScenarioType::Concurrent,
            None,
        )];
        
        let events: Vec<event::Cucumber<TestWorld>> = storage.start_scenarios(runnable).collect();
        assert_eq!(events.len(), 1); // Should have feature started event
    }

    #[test]
    fn test_finished_rules_and_features_feature_scenario_finished() {
        let (_, receiver) = mpsc::unbounded();
        let mut storage = FinishedRulesAndFeatures::new(receiver);
        
        let feature = Source::new(gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            description: None,
            background: None,
            scenarios: vec![
                gherkin::Scenario {
                    tags: vec![],
                    keyword: "Scenario".to_string(),
                    name: "Scenario 1".to_string(),
                    span: gherkin::Span { start: 0, end: 0 },
                    position: gherkin::LineCol { line: 1, col: 1 },
                    description: None,
                    steps: vec![],
                    examples: vec![],
                },
            ],
            rules: vec![],
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        });
        
        // Start tracking this feature
        storage.features_scenarios_count.insert(feature.clone(), 0);
        
        // Finish the scenario (not retried)
        let result: Option<event::Cucumber<()>> = storage.feature_scenario_finished(feature, false);
        assert!(result.is_some());
        
        // Feature should be removed from tracking
        assert!(storage.features_scenarios_count.is_empty());
    }

    #[test]
    fn test_finished_rules_and_features_feature_scenario_finished_retried() {
        let (_, receiver) = mpsc::unbounded();
        let mut storage = FinishedRulesAndFeatures::new(receiver);
        
        let feature = Source::new(gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
        });
        
        // Finish a retried scenario - should return None
        let result: Option<event::Cucumber<()>> = storage.feature_scenario_finished(feature, true);
        assert!(result.is_none());
    }

    #[test]
    fn test_finished_rules_and_features_finish_all() {
        let (_, receiver) = mpsc::unbounded();
        let mut storage = FinishedRulesAndFeatures::new(receiver);
        
        let feature = Source::new(gherkin::Feature {
            tags: vec![],
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
        });
        
        // Add some unfinished features and rules
        storage.features_scenarios_count.insert(feature.clone(), 0);
        
        let events: Vec<event::Cucumber<()>> = storage.finish_all_rules_and_features().collect();
        assert_eq!(events.len(), 1); // Should have one feature finished event
        
        // Storage should be empty after finishing all
        assert!(storage.features_scenarios_count.is_empty());
        assert!(storage.rule_scenarios_count.is_empty());
    }
}