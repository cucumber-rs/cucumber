//! Feature-level events.

use super::{RetryableScenario, Rule, Source};

/// Event specific to a particular [Feature].
///
/// [Feature]: https://cucumber.io/docs/gherkin/reference#feature
#[derive(Debug)]
pub enum Feature<World> {
    /// [`Feature`] execution being started.
    ///
    /// [`Feature`]: gherkin::Feature
    Started,

    /// [`Rule`] event.
    Rule(Source<gherkin::Rule>, Rule<World>),

    /// [`Scenario`] event.
    Scenario(Source<gherkin::Scenario>, RetryableScenario<World>),

    /// [`Feature`] execution being finished.
    ///
    /// [`Feature`]: gherkin::Feature
    Finished,
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for Feature<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Rule(r, ev) => Self::Rule(r.clone(), ev.clone()),
            Self::Scenario(s, ev) => Self::Scenario(s.clone(), ev.clone()),
            Self::Finished => Self::Finished,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Scenario, Retries};

    #[derive(Debug, Clone)]
    struct TestWorld {
        value: String,
    }

    fn create_test_rule() -> gherkin::Rule {
        gherkin::Rule {
            keyword: "Rule".to_string(),
            name: "Test Rule".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 10 },
            position: gherkin::LineCol { line: 1, col: 1 },
        }
    }

    fn create_test_scenario() -> gherkin::Scenario {
        gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 10 },
            position: gherkin::LineCol { line: 2, col: 1 },
        }
    }

    #[test]
    fn test_feature_started_event() {
        let event: Feature<TestWorld> = Feature::Started;
        assert!(matches!(event, Feature::Started));
    }

    #[test]
    fn test_feature_finished_event() {
        let event: Feature<TestWorld> = Feature::Finished;
        assert!(matches!(event, Feature::Finished));
    }

    #[test]
    fn test_feature_rule_event() {
        let rule = create_test_rule();
        let rule_event = Rule::<TestWorld>::Started;
        let event = Feature::Rule(Source::new(rule.clone()), rule_event);
        
        match event {
            Feature::Rule(r, Rule::Started) => {
                assert_eq!(r.name, "Test Rule");
            },
            _ => panic!("Expected Feature::Rule with Started event"),
        }
    }

    #[test]
    fn test_feature_scenario_event() {
        let scenario = create_test_scenario();
        let scenario_event = RetryableScenario {
            event: Scenario::<TestWorld>::Started,
            retries: None,
        };
        let event = Feature::Scenario(Source::new(scenario.clone()), scenario_event);
        
        match event {
            Feature::Scenario(s, RetryableScenario { event: Scenario::Started, retries }) => {
                assert_eq!(s.name, "Test Scenario");
                assert!(retries.is_none());
            },
            _ => panic!("Expected Feature::Scenario with Started event"),
        }
    }

    #[test]
    fn test_feature_scenario_with_retries() {
        let scenario = create_test_scenario();
        let retries = Retries { current: 1, left: 2 };
        let scenario_event = RetryableScenario {
            event: Scenario::<TestWorld>::Finished,
            retries: Some(retries),
        };
        let event = Feature::Scenario(Source::new(scenario), scenario_event);
        
        match event {
            Feature::Scenario(_, RetryableScenario { event: Scenario::Finished, retries }) => {
                assert!(retries.is_some());
                let r = retries.unwrap();
                assert_eq!(r.current, 1);
                assert_eq!(r.left, 2);
            },
            _ => panic!("Expected Feature::Scenario with Finished event and retries"),
        }
    }

    #[test]
    fn test_feature_clone() {
        let events = vec![
            Feature::<TestWorld>::Started,
            Feature::Finished,
            Feature::Rule(Source::new(create_test_rule()), Rule::Started),
            Feature::Rule(Source::new(create_test_rule()), Rule::Finished),
            Feature::Scenario(
                Source::new(create_test_scenario()),
                RetryableScenario {
                    event: Scenario::Started,
                    retries: None,
                }
            ),
        ];

        for event in events {
            let cloned = event.clone();
            match (&event, &cloned) {
                (Feature::Started, Feature::Started) => {},
                (Feature::Finished, Feature::Finished) => {},
                (Feature::Rule(r1, e1), Feature::Rule(r2, e2)) => {
                    assert_eq!(r1.name, r2.name);
                    match (e1, e2) {
                        (Rule::Started, Rule::Started) => {},
                        (Rule::Finished, Rule::Finished) => {},
                        _ => {},
                    }
                },
                (Feature::Scenario(s1, e1), Feature::Scenario(s2, e2)) => {
                    assert_eq!(s1.name, s2.name);
                    assert_eq!(e1.retries, e2.retries);
                },
                _ => panic!("Clone produced different variant"),
            }
        }
    }

    #[test]
    fn test_nested_rule_scenarios() {
        let rule = create_test_rule();
        let scenario = create_test_scenario();
        
        // Create a Rule event containing a Scenario
        let rule_event = Rule::Scenario(
            Source::new(scenario.clone()),
            RetryableScenario {
                event: Scenario::<TestWorld>::Started,
                retries: None,
            }
        );
        
        let feature_event = Feature::Rule(Source::new(rule), rule_event);
        
        match feature_event {
            Feature::Rule(r, Rule::Scenario(s, _)) => {
                assert_eq!(r.name, "Test Rule");
                assert_eq!(s.name, "Test Scenario");
            },
            _ => panic!("Expected nested Rule::Scenario event"),
        }
    }

    #[test]
    fn test_feature_event_variants_coverage() {
        // Test all possible Feature event variants
        let rule = create_test_rule();
        let scenario = create_test_scenario();
        
        let variants: Vec<Feature<TestWorld>> = vec![
            Feature::Started,
            Feature::Rule(Source::new(rule.clone()), Rule::Started),
            Feature::Rule(Source::new(rule.clone()), Rule::Finished),
            Feature::Scenario(
                Source::new(scenario.clone()),
                RetryableScenario {
                    event: Scenario::Started,
                    retries: None,
                }
            ),
            Feature::Scenario(
                Source::new(scenario.clone()),
                RetryableScenario {
                    event: Scenario::Finished,
                    retries: Some(Retries { current: 1, left: 0 }),
                }
            ),
            Feature::Finished,
        ];
        
        // Verify each variant can be matched
        for variant in variants {
            match variant {
                Feature::Started => assert!(true),
                Feature::Rule(_, _) => assert!(true),
                Feature::Scenario(_, _) => assert!(true),
                Feature::Finished => assert!(true),
            }
        }
    }
}