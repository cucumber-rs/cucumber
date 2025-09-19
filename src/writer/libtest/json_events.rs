// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! JSON event types for libtest output format.

use std::time::Duration;

use derive_more::with_trait::From;
use serde::Serialize;

/// [`libtest`][1]'s JSON event.
///
/// This format isn't stable, so this implementation uses [implementation][1] as
/// a reference point.
///
/// [1]: https://bit.ly/3PrLtKC
#[derive(Clone, Debug, From, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LibTestJsonEvent {
    /// Event of test suite.
    Suite {
        /// [`SuiteEvent`]
        #[serde(flatten)]
        event: SuiteEvent,
    },

    /// Event of the test case.
    Test {
        /// [`TestEvent`]
        #[serde(flatten)]
        event: TestEvent,
    },
}

/// Test suite event.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum SuiteEvent {
    /// Test suite started.
    Started {
        /// Number of test cases. In our case, this is number of parsed
        /// [`Step`]s and [`Parser`] errors.
        ///
        /// [`Parser`]: crate::Parser
        /// [`Step`]: gherkin::Step
        test_count: usize,
    },

    /// Test suite finished without errors.
    Ok {
        /// Execution results.
        #[serde(flatten)]
        results: SuiteResults,
    },

    /// Test suite encountered errors during the execution.
    Failed {
        /// Execution results.
        #[serde(flatten)]
        results: SuiteResults,
    },
}

/// Test suite execution results.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct SuiteResults {
    /// Number of passed test cases.
    pub passed: usize,

    /// Number of failed test cases.
    pub failed: usize,

    /// Number of ignored test cases.
    pub ignored: usize,

    /// Number of measured benches.
    pub measured: usize,

    // TODO: Figure out a way to actually report this.
    /// Number of filtered out test cases.
    pub filtered_out: usize,

    /// Test suite execution time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_time: Option<f64>,
}

/// Test case event.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TestEvent {
    /// Test case started.
    Started(TestEventInner),

    /// Test case finished successfully.
    Ok(TestEventInner),

    /// Test case failed.
    Failed(TestEventInner),

    /// Test case ignored.
    Ignored(TestEventInner),

    /// Test case timed out.
    Timeout(TestEventInner),
}

impl TestEvent {
    /// Creates a new [`TestEvent::Started`].
    pub const fn started(name: String) -> Self {
        Self::Started(TestEventInner::new(name))
    }

    /// Creates a new [`TestEvent::Ok`].
    pub fn ok(name: String, exec_time: Option<Duration>) -> Self {
        Self::Ok(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Creates a new [`TestEvent::Failed`].
    pub fn failed(name: String, exec_time: Option<Duration>) -> Self {
        Self::Failed(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Creates a new [`TestEvent::Ignored`].
    pub fn ignored(name: String, exec_time: Option<Duration>) -> Self {
        Self::Ignored(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Creates a new [`TestEvent::Timeout`].
    #[expect(dead_code, reason = "API uniformity")]
    pub fn timeout(name: String, exec_time: Option<Duration>) -> Self {
        Self::Timeout(TestEventInner::new(name).with_exec_time(exec_time))
    }

    /// Adds a [`TestEventInner::stdout`].
    pub fn with_stdout(self, mut stdout: String) -> Self {
        if !stdout.ends_with('\n') {
            stdout.push('\n');
        }

        match self {
            Self::Started(inner) => Self::Started(inner.with_stdout(stdout)),
            Self::Ok(inner) => Self::Ok(inner.with_stdout(stdout)),
            Self::Failed(inner) => Self::Failed(inner.with_stdout(stdout)),
            Self::Ignored(inner) => Self::Ignored(inner.with_stdout(stdout)),
            Self::Timeout(inner) => Self::Timeout(inner.with_stdout(stdout)),
        }
    }
}

/// Inner value of a [`TestEvent`].
#[derive(Clone, Debug, Serialize)]
pub struct TestEventInner {
    /// Name of this test case.
    pub name: String,

    /// [`Stdout`] of this test case.
    ///
    /// [`Stdout`]: std::io::Stdout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,

    /// [`Stderr`] of this test case.
    ///
    /// Isn't actually used, as [IntelliJ Rust][1] ignores it.
    ///
    /// [1]: https://github.com/intellij-rust/intellij-rust/issues/9041
    /// [`Stderr`]: std::io::Stderr
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,

    /// Test case execution time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_time: Option<f64>,
}

impl TestEventInner {
    /// Creates a new [`TestEventInner`].
    pub const fn new(name: String) -> Self {
        Self { name, stdout: None, stderr: None, exec_time: None }
    }

    /// Adds a [`TestEventInner::exec_time`].
    pub fn with_exec_time(mut self, exec_time: Option<Duration>) -> Self {
        self.exec_time = exec_time.as_ref().map(Duration::as_secs_f64);
        self
    }

    /// Adds a [`TestEventInner::stdout`].
    pub fn with_stdout(mut self, stdout: String) -> Self {
        self.stdout = Some(stdout);
        self
    }

    /// Adds a [`TestEventInner::stderr`].
    #[expect(dead_code, reason = "API completeness")]
    pub fn with_stderr(mut self, stderr: String) -> Self {
        self.stderr = Some(stderr);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    mod suite_event_tests {
        use super::*;

        #[test]
        fn suite_event_started_serialization() {
            let event = SuiteEvent::Started { test_count: 42 };
            let json = serde_json::to_string(&event).expect("should serialize");
            
            assert!(json.contains("\"event\":\"started\""));
            assert!(json.contains("\"test_count\":42"));
        }

        #[test]
        fn suite_event_ok_serialization() {
            let results = SuiteResults {
                passed: 10,
                failed: 0,
                ignored: 2,
                measured: 0,
                filtered_out: 0,
                exec_time: Some(1.5),
            };
            let event = SuiteEvent::Ok { results };
            let json = serde_json::to_string(&event).expect("should serialize");
            
            assert!(json.contains("\"event\":\"ok\""));
            assert!(json.contains("\"passed\":10"));
            assert!(json.contains("\"failed\":0"));
            assert!(json.contains("\"exec_time\":1.5"));
        }

        #[test]
        fn suite_event_failed_serialization() {
            let results = SuiteResults {
                passed: 8,
                failed: 2,
                ignored: 1,
                measured: 0,
                filtered_out: 0,
                exec_time: None,
            };
            let event = SuiteEvent::Failed { results };
            let json = serde_json::to_string(&event).expect("should serialize");
            
            assert!(json.contains("\"event\":\"failed\""));
            assert!(json.contains("\"passed\":8"));
            assert!(json.contains("\"failed\":2"));
            // exec_time should be omitted when None
            assert!(!json.contains("exec_time"));
        }
    }

    mod test_event_tests {
        use super::*;

        #[test]
        fn test_event_started_creation() {
            let event = TestEvent::started("test_name".to_string());
            
            if let TestEvent::Started(inner) = event {
                assert_eq!(inner.name, "test_name");
                assert!(inner.stdout.is_none());
                assert!(inner.exec_time.is_none());
            } else {
                panic!("Expected TestEvent::Started");
            }
        }

        #[test]
        fn test_event_ok_creation() {
            let duration = Duration::from_millis(1500);
            let event = TestEvent::ok("test_name".to_string(), Some(duration));
            
            if let TestEvent::Ok(inner) = event {
                assert_eq!(inner.name, "test_name");
                assert_eq!(inner.exec_time, Some(1.5));
            } else {
                panic!("Expected TestEvent::Ok");
            }
        }

        #[test]
        fn test_event_failed_creation() {
            let event = TestEvent::failed("test_name".to_string(), None);
            
            if let TestEvent::Failed(inner) = event {
                assert_eq!(inner.name, "test_name");
                assert!(inner.exec_time.is_none());
            } else {
                panic!("Expected TestEvent::Failed");
            }
        }

        #[test]
        fn test_event_ignored_creation() {
            let duration = Duration::from_secs(2);
            let event = TestEvent::ignored("test_name".to_string(), Some(duration));
            
            if let TestEvent::Ignored(inner) = event {
                assert_eq!(inner.name, "test_name");
                assert_eq!(inner.exec_time, Some(2.0));
            } else {
                panic!("Expected TestEvent::Ignored");
            }
        }

        #[test]
        fn test_event_timeout_creation() {
            let event = TestEvent::timeout("test_name".to_string(), None);
            
            if let TestEvent::Timeout(inner) = event {
                assert_eq!(inner.name, "test_name");
                assert!(inner.exec_time.is_none());
            } else {
                panic!("Expected TestEvent::Timeout");
            }
        }

        #[test]
        fn test_event_with_stdout() {
            let event = TestEvent::started("test".to_string())
                .with_stdout("output without newline");
            
            if let TestEvent::Started(inner) = event {
                assert_eq!(inner.stdout, Some("output without newline\n".to_string()));
            } else {
                panic!("Expected TestEvent::Started");
            }
        }

        #[test]
        fn test_event_with_stdout_already_has_newline() {
            let event = TestEvent::started("test".to_string())
                .with_stdout("output with newline\n");
            
            if let TestEvent::Started(inner) = event {
                assert_eq!(inner.stdout, Some("output with newline\n".to_string()));
            } else {
                panic!("Expected TestEvent::Started");
            }
        }

        #[test]
        fn test_event_serialization() {
            let event = TestEvent::started("my_test".to_string())
                .with_stdout("test output");
            let json = serde_json::to_string(&event).expect("should serialize");
            
            assert!(json.contains("\"event\":\"started\""));
            assert!(json.contains("\"name\":\"my_test\""));
            assert!(json.contains("\"stdout\":\"test output\\n\""));
        }
    }

    mod test_event_inner_tests {
        use super::*;

        #[test]
        fn test_event_inner_new() {
            let inner = TestEventInner::new("test_name".to_string());
            
            assert_eq!(inner.name, "test_name");
            assert!(inner.stdout.is_none());
            assert!(inner.stderr.is_none());
            assert!(inner.exec_time.is_none());
        }

        #[test]
        fn test_event_inner_with_exec_time() {
            let duration = Duration::from_millis(2500);
            let inner = TestEventInner::new("test".to_string())
                .with_exec_time(Some(duration));
            
            assert_eq!(inner.exec_time, Some(2.5));
        }

        #[test]
        fn test_event_inner_with_exec_time_none() {
            let inner = TestEventInner::new("test".to_string())
                .with_exec_time(None);
            
            assert!(inner.exec_time.is_none());
        }

        #[test]
        fn test_event_inner_with_stdout() {
            let inner = TestEventInner::new("test".to_string())
                .with_stdout("stdout content".to_string());
            
            assert_eq!(inner.stdout, Some("stdout content".to_string()));
        }

        #[test]
        fn test_event_inner_with_stderr() {
            let inner = TestEventInner::new("test".to_string())
                .with_stderr("stderr content".to_string());
            
            assert_eq!(inner.stderr, Some("stderr content".to_string()));
        }

        #[test]
        fn test_event_inner_method_chaining() {
            let duration = Duration::from_secs(1);
            let inner = TestEventInner::new("test".to_string())
                .with_exec_time(Some(duration))
                .with_stdout("output".to_string())
                .with_stderr("error".to_string());
            
            assert_eq!(inner.name, "test");
            assert_eq!(inner.exec_time, Some(1.0));
            assert_eq!(inner.stdout, Some("output".to_string()));
            assert_eq!(inner.stderr, Some("error".to_string()));
        }
    }

    mod libtest_json_event_tests {
        use super::*;

        #[test]
        fn libtest_json_event_suite_from() {
            let suite_event = SuiteEvent::Started { test_count: 5 };
            let json_event: LibTestJsonEvent = suite_event.into();
            
            if let LibTestJsonEvent::Suite { event } = json_event {
                if let SuiteEvent::Started { test_count } = event {
                    assert_eq!(test_count, 5);
                } else {
                    panic!("Expected SuiteEvent::Started");
                }
            } else {
                panic!("Expected LibTestJsonEvent::Suite");
            }
        }

        #[test]
        fn libtest_json_event_test_from() {
            let test_event = TestEvent::started("test".to_string());
            let json_event: LibTestJsonEvent = test_event.into();
            
            if let LibTestJsonEvent::Test { event } = json_event {
                if let TestEvent::Started(inner) = event {
                    assert_eq!(inner.name, "test");
                } else {
                    panic!("Expected TestEvent::Started");
                }
            } else {
                panic!("Expected LibTestJsonEvent::Test");
            }
        }

        #[test]
        fn libtest_json_event_serialization() {
            let suite_event = SuiteEvent::Started { test_count: 3 };
            let json_event: LibTestJsonEvent = suite_event.into();
            let json = serde_json::to_string(&json_event).expect("should serialize");
            
            assert!(json.contains("\"type\":\"suite\""));
            assert!(json.contains("\"event\":\"started\""));
            assert!(json.contains("\"test_count\":3"));
        }

        #[test]
        fn libtest_json_event_clone() {
            let original = LibTestJsonEvent::Suite {
                event: SuiteEvent::Started { test_count: 10 }
            };
            let cloned = original.clone();
            
            // Both should serialize to the same JSON
            let original_json = serde_json::to_string(&original).expect("should serialize");
            let cloned_json = serde_json::to_string(&cloned).expect("should serialize");
            assert_eq!(original_json, cloned_json);
        }
    }

    mod suite_results_tests {
        use super::*;

        #[test]
        fn suite_results_default_values() {
            let results = SuiteResults {
                passed: 0,
                failed: 0,
                ignored: 0,
                measured: 0,
                filtered_out: 0,
                exec_time: None,
            };
            
            let json = serde_json::to_string(&results).expect("should serialize");
            
            // exec_time should not appear in JSON when None
            assert!(!json.contains("exec_time"));
            assert!(json.contains("\"passed\":0"));
            assert!(json.contains("\"failed\":0"));
        }

        #[test]
        fn suite_results_with_exec_time() {
            let results = SuiteResults {
                passed: 5,
                failed: 1,
                ignored: 2,
                measured: 0,
                filtered_out: 0,
                exec_time: Some(3.14159),
            };
            
            let json = serde_json::to_string(&results).expect("should serialize");
            
            assert!(json.contains("\"exec_time\":3.14159"));
            assert!(json.contains("\"passed\":5"));
            assert!(json.contains("\"failed\":1"));
            assert!(json.contains("\"ignored\":2"));
        }

        #[test]
        fn suite_results_copy_trait() {
            let results1 = SuiteResults {
                passed: 1,
                failed: 2,
                ignored: 3,
                measured: 4,
                filtered_out: 5,
                exec_time: Some(6.0),
            };
            
            let results2 = results1; // Should work due to Copy trait
            
            assert_eq!(results1.passed, results2.passed);
            assert_eq!(results1.exec_time, results2.exec_time);
        }
    }
}