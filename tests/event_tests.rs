use cucumber::event::*;
use cucumber::step;
use std::sync::Arc;

#[derive(std::fmt::Debug, Clone, PartialEq, Eq)]
struct TestWorld;

#[test]
fn test_event_new_creates_event_correctly() {
    let value = 42;
    let event = Event::new(value);
    
    assert_eq!(event.value, 42);
    assert_eq!(*event, 42);
    
    #[cfg(feature = "timestamps")]
    {
        let now = std::time::SystemTime::now();
        let diff = now.duration_since(event.at).unwrap_or_else(|_| event.at.duration_since(now).unwrap());
        assert!(diff.as_secs() < 1, "Event timestamp should be very recent");
    }
}

#[test]
fn test_event_into_inner() {
    let event = Event::new("test");
    let value = event.into_inner();
    assert_eq!(value, "test");
}

#[test]
fn test_event_split() {
    let event = Event::new(123);
    let (value, metadata) = event.split();
    
    assert_eq!(value, 123);
    assert_eq!(metadata.value, ());
    
    #[cfg(feature = "timestamps")]
    {
        let now = std::time::SystemTime::now();
        let diff = now.duration_since(metadata.at).unwrap_or_else(|_| metadata.at.duration_since(now).unwrap());
        assert!(diff.as_secs() < 1);
    }
}

#[test]
fn test_event_insert() {
    let event = Event::new(123);
    let new_event = event.insert("hello");
    
    assert_eq!(new_event.value, "hello");
    assert_eq!(*new_event, "hello");
}

#[test]
fn test_event_map() {
    let event = Event::new(5);
    let mapped = event.map(|x| x * 2);
    
    assert_eq!(mapped.value, 10);
    assert_eq!(*mapped, 10);
}

#[test]
fn test_event_replace() {
    let event = Event::new("original");
    let (old_value, new_event) = event.replace(999);
    
    assert_eq!(old_value, "original");
    assert_eq!(new_event.value, 999);
    assert_eq!(*new_event, 999);
}

#[test]
fn test_metadata_wrap() {
    let event = Event::new(42);
    let (_, metadata) = event.split();
    let wrapped = metadata.wrap("wrapped");
    
    assert_eq!(wrapped.value, "wrapped");
    assert_eq!(*wrapped, "wrapped");
}

#[test]
fn test_event_deref_and_deref_mut() {
    let mut event = Event::new(vec![1, 2, 3]);
    
    // Test deref
    assert_eq!(event.len(), 3);
    assert_eq!(event[0], 1);
    
    // Test deref_mut
    event.push(4);
    assert_eq!(event.len(), 4);
    assert_eq!(*event, vec![1, 2, 3, 4]);
}

#[test]
fn test_event_as_ref() {
    let event = Event::new(String::from("test"));
    let as_ref: &str = event.as_ref();
    assert_eq!(as_ref, "test");
}

#[test] 
fn test_retries_initial() {
    let retries = Retries::initial(5);
    assert_eq!(retries.current, 0);
    assert_eq!(retries.left, 5);
}

#[test]
fn test_retries_exhausted() {
    let retries = Retries::initial(0);
    assert_eq!(retries.current, 0);
    assert_eq!(retries.left, 0);
}

#[test]
fn test_retries_next_try() {
    let retries = Retries::initial(3);
    
    let next = retries.next_try().unwrap();
    assert_eq!(next.current, 1);
    assert_eq!(next.left, 2);
    
    // Chain multiple next_try calls
    let next2 = next.next_try().unwrap();
    assert_eq!(next2.current, 2);
    assert_eq!(next2.left, 1);
    
    let final_retry = next2.next_try().unwrap();
    assert_eq!(final_retry.current, 3);
    assert_eq!(final_retry.left, 0);
    
    // Should be None now
    assert!(final_retry.next_try().is_none());
}

#[test]
fn test_retries_partial_eq() {
    let retries1 = Retries { current: 1, left: 2 };
    let retries2 = Retries { current: 1, left: 2 };
    let retries3 = Retries { current: 2, left: 1 };
    
    assert_eq!(retries1, retries2);
    assert_ne!(retries1, retries3);
}

#[test]
fn test_retries_hash() {
    use std::collections::HashMap;
    
    let retries1 = Retries { current: 1, left: 2 };
    let retries2 = Retries { current: 1, left: 2 };
    
    let mut map = HashMap::new();
    map.insert(retries1, "value");
    
    assert_eq!(map.get(&retries2), Some(&"value"));
}

#[test]
fn test_cucumber_event_started_finished() {
    let started = Cucumber::<TestWorld>::Started;
    let finished = Cucumber::<TestWorld>::Finished;
    
    assert!(matches!(started, Cucumber::Started));
    assert!(matches!(finished, Cucumber::Finished));
    
    // Test cloning
    let started_clone = started.clone();
    assert!(matches!(started_clone, Cucumber::Started));
}

#[test]
fn test_cucumber_event_feature() {
    // Test event structure with minimal creation
    let feature_event = Feature::<TestWorld>::Started;
    assert!(matches!(feature_event, Feature::Started));
}

#[test]
fn test_feature_events() {
    let started = Feature::<TestWorld>::Started;
    let finished = Feature::<TestWorld>::Finished;
    
    assert!(matches!(started, Feature::Started));
    assert!(matches!(finished, Feature::Finished));
    
    // Test cloning
    let started_clone = started.clone();
    assert!(matches!(started_clone, Feature::Started));
}

#[test]
fn test_rule_events() {
    let started = Rule::<TestWorld>::Started;
    let finished = Rule::<TestWorld>::Finished;
    
    assert!(matches!(started, Rule::Started));
    assert!(matches!(finished, Rule::Finished));
    
    // Test cloning
    let started_clone = started.clone();
    assert!(matches!(started_clone, Rule::Started));
}

#[test]
fn test_step_events() {
    let started = Step::<TestWorld>::Started;
    let skipped = Step::<TestWorld>::Skipped;
    
    assert!(matches!(started, Step::Started));
    assert!(matches!(skipped, Step::Skipped));
    
    // Test cloning
    let started_clone = started.clone();
    assert!(matches!(started_clone, Step::Started));
}

#[test]
fn test_step_error_types() {
    let not_found = StepError::NotFound;
    let ambiguous = StepError::AmbiguousMatch(step::AmbiguousMatchError {
        possible_matches: vec![],
    });
    let panic_err = StepError::Panic(Arc::new("panic message"));
    
    assert!(matches!(not_found, StepError::NotFound));
    assert!(matches!(ambiguous, StepError::AmbiguousMatch(_)));
    assert!(matches!(panic_err, StepError::Panic(_)));
    
    // Test error display
    assert!(not_found.to_string().contains("doesn't match"));
    assert!(ambiguous.to_string().contains("ambiguous"));
}

#[test]
fn test_hook_type() {
    assert!(matches!(HookType::Before, HookType::Before));
    assert!(matches!(HookType::After, HookType::After));
}

#[test]
fn test_hook_events() {
    let started = Hook::<TestWorld>::Started;
    let passed = Hook::<TestWorld>::Passed;
    let failed = Hook::<TestWorld>::Failed(Some(Arc::new(TestWorld)), Arc::new("error"));
    
    assert!(matches!(started, Hook::Started));
    assert!(matches!(passed, Hook::Passed));
    assert!(matches!(failed, Hook::Failed(_, _)));
    
    // Test cloning
    let started_clone = started.clone();
    assert!(matches!(started_clone, Hook::Started));
}

#[test]
fn test_scenario_events() {
    let started = Scenario::<TestWorld>::Started;
    let hook_event = Scenario::<TestWorld>::Hook(HookType::Before, Hook::Started);
    
    assert!(matches!(started, Scenario::Started));
    assert!(matches!(hook_event, Scenario::Hook(_, _)));
    
    // Test cloning
    let started_clone = started.clone();
    assert!(matches!(started_clone, Scenario::Started));
}

#[test]
fn test_retryable_scenario() {
    let scenario_event = Scenario::<TestWorld>::Started;
    let retryable = RetryableScenario {
        event: scenario_event,
        retries: Some(Retries::initial(2)),
    };
    
    assert!(matches!(retryable.event, Scenario::Started));
    assert_eq!(retryable.retries.unwrap().left, 2);
    
    // Test cloning
    let retryable_clone = retryable.clone();
    assert!(matches!(retryable_clone.event, Scenario::Started));
    assert_eq!(retryable_clone.retries.unwrap().left, 2);
}

#[test]
fn test_event_chain_transformations() {
    // Test chaining multiple transformations
    let event = Event::new(5)
        .map(|x| x * 2)
        .map(|x| x.to_string())
        .map(|s| format!("Value: {}", s));
    
    assert_eq!(event.value, "Value: 10");
    assert_eq!(*event, "Value: 10");
}

#[test]
fn test_metadata_preserves_timestamp() {
    let original = Event::new(42);
    let (_, metadata) = original.split();
    let new_event = metadata.wrap("new value");
    
    #[cfg(feature = "timestamps")]
    {
        // Timestamps should be preserved when using metadata
        assert_eq!(original.at, new_event.at);
    }
    
    assert_eq!(new_event.value, "new value");
}

#[test]
fn test_step_error_coercion() {
    // Test that our step errors can be properly displayed
    let errors = vec![
        StepError::NotFound,
        StepError::AmbiguousMatch(step::AmbiguousMatchError { possible_matches: vec![] }),
        StepError::Panic(Arc::new("test panic")),
    ];
    
    for error in errors {
        let error_string = error.to_string();
        assert!(!error_string.is_empty());
        assert!(error_string.len() > 5); // Should have meaningful content
    }
}

#[test]
fn test_step_failed_structure() {
    // Test Step::Failed with proper parameters
    let failed_step = Step::<TestWorld>::Failed(
        None, 
        Some(step::Location {
            path: "test.feature",
            line: 10,
            column: 1,
        }),
        Some(Arc::new(TestWorld)),
        StepError::NotFound
    );
    
    if let Step::Failed(_, loc, world, err) = failed_step {
        assert!(loc.is_some());
        assert_eq!(loc.unwrap().line, 10);
        assert!(world.is_some());
        assert!(matches!(err, StepError::NotFound));
    }
}

#[cfg(feature = "timestamps")]
#[test]
fn test_timestamp_feature() {
    let event1 = Event::new(1);
    std::thread::sleep(std::time::Duration::from_millis(1));
    let event2 = Event::new(2);
    
    assert!(event2.at > event1.at, "Second event should have later timestamp");
    
    let duration = event2.at.duration_since(event1.at).unwrap();
    assert!(duration.as_millis() >= 1, "Events should be at least 1ms apart");
}

#[test]
fn test_event_debug_formatting() {
    let event = Event::new("debug test");
    let debug_str = format!("{:?}", event);
    
    assert!(debug_str.contains("debug test"));
    assert!(debug_str.contains("Event"));
    
    #[cfg(feature = "timestamps")]
    assert!(debug_str.contains("at:"));
}